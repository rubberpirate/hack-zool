use crate::{WalletConfig, cache::MemoryCache};
use async_trait::async_trait;
use rand::RngCore;
use rand10::SeedableRng;
use rusqlite::{Connection, OptionalExtension, params};
use secrecy::{ExposeSecret, SecretVec};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    num::NonZeroU32,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Semaphore;
use tonic::transport::{Channel, Endpoint};
use zcash_client_backend::{
    data_api::{
        Account, AccountBirthday, TransactionDataRequest, TransactionStatus, WalletRead,
        WalletWrite,
        wallet::{self, ConfirmationsPolicy, SpendingKeys},
    },
    fees::StandardFeeRule,
    proto::service::{
        BlockId, ChainSpec, Empty, RawTransaction, TxFilter,
        compact_tx_streamer_client::CompactTxStreamerClient,
    },
    util::SystemClock,
    wallet::OvkPolicy,
};
use zcash_client_sqlite::{AccountUuid, WalletDb, wallet::init::init_wallet_db};
use zcash_keys::{
    address::{Address, UnifiedAddress},
    keys::{UnifiedAddressRequest, UnifiedSpendingKey},
};
use zcash_primitives::transaction::Transaction;
use zcash_proofs::prover::LocalTxProver;
use zcash_protocol::{
    ShieldedPool, TxId,
    consensus::{self, BlockHeight, BranchId, Parameters},
    value::Zatoshis,
};
use zsplit_domain::{
    BalanceSnapshot, ChainTxStatus, DepositAddress, DomainError, FundingCommand, PayoutEngine,
    PreparedTransaction, RoomId,
};

type Db = WalletDb<Connection, consensus::Network, SystemClock, rand_chacha::ChaCha20Rng>;
struct State {
    db: Db,
    journal: Connection,
    config: WalletConfig,
    seed: SecretVec<u8>,
}

/// Serialized native wallet writer. SDK scanning and proving run on a bounded blocking worker.
/// An OS lock prevents a competing process from opening this wallet through this service.
#[derive(Clone)]
pub struct NativeWallet {
    state: Arc<Mutex<State>>,
    gate: Arc<Semaphore>,
    client: CompactTxStreamerClient<Channel>,
    _lock: Arc<File>,
}
fn sdk_error(_: impl std::fmt::Debug) -> DomainError {
    DomainError::Unavailable
}
fn storage_error(_: impl std::fmt::Debug) -> DomainError {
    DomainError::Storage
}
fn policy(n: u32) -> Result<ConfirmationsPolicy, DomainError> {
    Ok(ConfirmationsPolicy::new_symmetrical(
        NonZeroU32::new(n).ok_or(DomainError::InvalidInput)?,
    ))
}
fn active_output_pool(height: BlockHeight) -> ShieldedPool {
    if consensus::Network::TestNetwork.is_nu_active(consensus::NetworkUpgrade::Nu6_3, height) {
        ShieldedPool::Ironwood
    } else {
        ShieldedPool::Orchard
    }
}
fn validate_chain_info(
    info: &zcash_client_backend::proto::service::LightdInfo,
) -> Result<(), DomainError> {
    if !matches!(info.chain_name.as_str(), "test" | "testnet") {
        return Err(DomainError::NetworkMismatch);
    }
    let height = u32::try_from(info.block_height).map_err(sdk_error)?;
    let actual = u32::from_str_radix(info.consensus_branch_id.trim_start_matches("0x"), 16)
        .map_err(|_| DomainError::NetworkMismatch)?;
    let expected: u32 = BranchId::for_height(
        &consensus::Network::TestNetwork,
        BlockHeight::from_u32(height),
    )
    .into();
    if actual != expected {
        return Err(DomainError::NetworkMismatch);
    }
    Ok(())
}
fn private_file(path: &Path) -> Result<File, DomainError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let f = options.open(path).map_err(storage_error)?;
    check_private(&f)?;
    Ok(f)
}
fn check_private(f: &File) -> Result<(), DomainError> {
    if !f.metadata().map_err(storage_error)?.is_file() {
        return Err(DomainError::InvalidInput);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if f.metadata().map_err(storage_error)?.permissions().mode() & 0o077 != 0 {
            return Err(DomainError::InvalidInput);
        }
    }
    Ok(())
}
fn load_seed(path: &Path) -> Result<SecretVec<u8>, DomainError> {
    let mut f = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(storage_error)?;
    check_private(&f)?;
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    Read::by_ref(&mut f)
        .take(253)
        .read_to_end(&mut bytes)
        .map_err(storage_error)?;
    if !(32..=252).contains(&bytes.len()) {
        return Err(DomainError::InvalidInput);
    }
    Ok(SecretVec::new(bytes.to_vec()))
}
impl NativeWallet {
    /// Creates a raw coordinator seed in a NEW protected file. Never replaces an existing seed.
    pub fn create_seed_file(path: &Path) -> Result<(), DomainError> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(path).map_err(storage_error)?;
        let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
        rand::rngs::OsRng
            .try_fill_bytes(bytes.as_mut())
            .map_err(sdk_error)?;
        f.write_all(bytes.as_ref()).map_err(storage_error)?;
        f.sync_all().map_err(storage_error)?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(storage_error)?;
        }
        Ok(())
    }
    pub async fn open(config: WalletConfig) -> Result<Self, DomainError> {
        config.validate()?;
        let endpoint = Endpoint::from_shared(config.lightwalletd_url.clone())
            .map_err(sdk_error)?
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120));
        let endpoint = if endpoint.uri().scheme_str() == Some("https") {
            endpoint
                .tls_config(tonic::transport::ClientTlsConfig::new().with_webpki_roots())
                .map_err(sdk_error)?
        } else {
            endpoint
        };
        let channel = endpoint.connect().await.map_err(sdk_error)?;
        let mut client = CompactTxStreamerClient::new(channel);
        let info = client
            .get_lightd_info(Empty {})
            .await
            .map_err(sdk_error)?
            .into_inner();
        validate_chain_info(&info)?;
        let (state, lock) = tokio::task::spawn_blocking(move || Self::initialize(config))
            .await
            .map_err(sdk_error)??;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            gate: Arc::new(Semaphore::new(1)),
            client,
            _lock: Arc::new(lock),
        })
    }
    fn initialize(config: WalletConfig) -> Result<(State, File), DomainError> {
        fs::create_dir_all(&config.data_dir).map_err(storage_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config.data_dir, fs::Permissions::from_mode(0o700))
                .map_err(storage_error)?;
        }
        let lock = private_file(&config.data_dir.join("wallet.lock"))?;
        lock.try_lock().map_err(|_| DomainError::Conflict)?;
        let seed = load_seed(&config.seed_file)?;
        let path = config.data_dir.join("wallet.sqlite3");
        drop(private_file(&path)?);
        let rng =
            rand_chacha::ChaCha20Rng::try_from_rng(&mut rand10::rngs::SysRng).map_err(sdk_error)?;
        let mut db = WalletDb::for_path(path, consensus::Network::TestNetwork, SystemClock, rng)
            .map_err(storage_error)?;
        init_wallet_db(&mut db, Some(SecretVec::new(seed.expose_secret().clone())))
            .map_err(sdk_error)?;
        for id in db.get_account_ids().map_err(sdk_error)? {
            if !db.validate_seed(id, &seed).map_err(sdk_error)? {
                return Err(DomainError::InvalidInput);
            }
        }
        let path = config.data_dir.join("intents.sqlite3");
        drop(private_file(&path)?);
        let journal = Connection::open(path).map_err(storage_error)?;
        journal.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS intents (id TEXT PRIMARY KEY, fingerprint BLOB NOT NULL, state TEXT NOT NULL CHECK(state IN ('preparing','prepared')), txid TEXT, raw BLOB); CREATE TABLE IF NOT EXISTS chain_observations (txid TEXT PRIMARY KEY, mined_height INTEGER);").map_err(storage_error)?;
        Ok((
            State {
                db,
                journal,
                config,
                seed,
            },
            lock,
        ))
    }
    async fn run<T, F>(&self, f: F) -> Result<T, DomainError>
    where
        T: Send + 'static,
        F: FnOnce(&mut State) -> Result<T, DomainError> + Send + 'static,
    {
        let permit =
            tokio::time::timeout(Duration::from_secs(15), self.gate.clone().acquire_owned())
                .await
                .map_err(|_| DomainError::Unavailable)?
                .map_err(|_| DomainError::Unavailable)?;
        let state = self.state.clone();
        let lock = self._lock.clone();
        tokio::task::spawn_blocking(move || {
            let (_permit, _lock) = (permit, lock);
            let mut state = state
                .lock()
                .map_err(|_| DomainError::ReconciliationRequired)?;
            f(&mut state)
        })
        .await
        .map_err(|_| DomainError::ReconciliationRequired)?
    }
}
impl State {
    fn account(&self, room: RoomId) -> Result<AccountUuid, DomainError> {
        let expected = format!("zsplit:{room}");
        for id in self.db.get_account_ids().map_err(sdk_error)? {
            if self
                .db
                .get_account(id)
                .map_err(sdk_error)?
                .is_some_and(|a| a.name() == Some(expected.as_str()))
            {
                return Ok(id);
            }
        }
        Err(DomainError::NotFound)
    }
    fn balance(&self, room: RoomId) -> Result<BalanceSnapshot, DomainError> {
        let id = self.account(room)?;
        let Some(summary) = self
            .db
            .get_wallet_summary(policy(self.config.confirmations)?)
            .map_err(sdk_error)?
        else {
            return Err(DomainError::Unavailable);
        };
        // Conservatively back new reservations only with spendable, confirmed Orchard notes.
        let spendable = summary
            .account_balances()
            .get(&id)
            .map(|b| {
                (b.orchard_balance().spendable_value() + b.ironwood_balance().spendable_value())
                    .map(u64::from)
                    .ok_or(DomainError::InvalidInput)
            })
            .transpose()?
            .unwrap_or(0);
        Ok(BalanceSnapshot {
            room_id: room,
            confirmed_zat: spendable,
            spendable_zat: spendable,
            chain_height: summary.chain_tip_height().into(),
            scanned_height: summary.fully_scanned_height().into(),
        })
    }
    fn prepared(&self, id: uuid::Uuid) -> Result<Option<PreparedTransaction>, DomainError> {
        type IntentRow = (String, Option<String>, Option<Vec<u8>>);
        let row: Option<IntentRow> = self
            .journal
            .query_row(
                "SELECT state,txid,raw FROM intents WHERE id=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(storage_error)?;
        match row {
            None => Ok(None),
            Some((s, Some(txid), Some(raw_transaction))) if s == "prepared" => {
                Ok(Some(PreparedTransaction {
                    payout_id: id,
                    txid,
                    raw_transaction,
                }))
            }
            Some(_) => Err(DomainError::ReconciliationRequired),
        }
    }
}
#[async_trait]
impl PayoutEngine for NativeWallet {
    async fn deposit_address(&self, room: RoomId) -> Result<DepositAddress, DomainError> {
        let mut client = self.client.clone();
        let height = client
            .get_latest_block(ChainSpec {})
            .await
            .map_err(sdk_error)?
            .into_inner()
            .height;
        let tree = client
            .get_tree_state(BlockId {
                height,
                hash: vec![],
            })
            .await
            .map_err(sdk_error)?
            .into_inner();
        let birthday = AccountBirthday::from_treestate(tree, None).map_err(sdk_error)?;

        self.run(move |s| {
            let id = match s.account(room) {
                Ok(id) => id,
                Err(DomainError::NotFound) => {
                    s.db.create_account(
                        &format!("zsplit:{room}"),
                        &s.seed,
                        &birthday,
                        Some("protected-coordinator-seed"),
                    )
                    .map_err(sdk_error)?
                    .0
                }
                Err(e) => return Err(e),
            };
            let account =
                s.db.get_account(id)
                    .map_err(sdk_error)?
                    .ok_or(DomainError::NotFound)?;
            let (address, _) = account
                .uivk()
                .find_address(
                    zip32::DiversifierIndex::from(0u32),
                    UnifiedAddressRequest::ORCHARD,
                )
                .map_err(sdk_error)?;
            Ok(DepositAddress {
                room_id: room,
                address: address.encode(&consensus::Network::TestNetwork),
                network: s.config.network,
                birthday_height: account.birthday_height().into(),
            })
        })
        .await
    }
    async fn sync(&self) -> Result<Vec<BalanceSnapshot>, DomainError> {
        let mut client = self.client.clone();
        let handle = tokio::runtime::Handle::current();
        self.run(move |s| {
            handle.block_on(async {
                let info = client
                    .get_lightd_info(Empty {})
                    .await
                    .map_err(sdk_error)?
                    .into_inner();
                validate_chain_info(&info)?;
                if s.db.get_account_ids().map_err(sdk_error)?.is_empty() {
                    return Ok(());
                }
                if let Err(e) = zcash_client_backend::sync::run(
                    &mut client,
                    &consensus::Network::TestNetwork,
                    &MemoryCache::default(),
                    &mut s.db,
                    100,
                )
                .await
                {
                    tracing::warn!(code = "sync_run_error", error = ?e);
                    return Err(sdk_error(e));
                }
                // SDK sync leaves complete transaction enhancement to the application.
                for request in s.db.transaction_data_requests().map_err(sdk_error)? {
                    let (id, enhance) = match request {
                        TransactionDataRequest::GetStatus(id) => (id, false),
                        TransactionDataRequest::Enhancement(id) => (id, true),
                    };
                    match client
                        .get_transaction(TxFilter {
                            block: None,
                            index: 0,
                            hash: id.as_ref().to_vec(),
                        })
                        .await
                    {
                        Ok(response) => {
                            let raw = response.into_inner();
                            let status = match raw.height {
                                0 | u64::MAX => TransactionStatus::NotInMainChain,
                                h => TransactionStatus::Mined(BlockHeight::from_u32(
                                    u32::try_from(h).map_err(sdk_error)?,
                                )),
                            };
                            if enhance {
                                let height = match status {
                                    TransactionStatus::Mined(h) => h,
                                    _ => {
                                        s.db.chain_height()
                                            .map_err(sdk_error)?
                                            .ok_or(DomainError::Unavailable)?
                                    }
                                };
                                let tx = match Transaction::read(
                                    raw.data.as_slice(),
                                    BranchId::for_height(&consensus::Network::TestNetwork, height),
                                ) {
                                    Ok(tx) => tx,
                                    Err(e) => {
                                        tracing::warn!(code = "tx_read_error", error = ?e, txid = %id, height = ?height);
                                        return Err(sdk_error(e));
                                    }
                                };
                                if tx.txid() != id {
                                    return Err(DomainError::Unavailable);
                                }
                                if let Err(e) = wallet::decrypt_and_store_transaction(
                                    &consensus::Network::TestNetwork,
                                    &mut s.db,
                                    &tx,
                                    None,
                                ) {
                                    tracing::warn!(code = "decrypt_and_store_error", error = ?e, txid = %id);
                                    return Err(sdk_error(e));
                                }
                            }
                            s.db.set_transaction_status(id, status).map_err(sdk_error)?;
                        }
                        Err(e) if e.code() == tonic::Code::NotFound => {
                            s.db.set_transaction_status(id, TransactionStatus::TxidNotRecognized)
                                .map_err(sdk_error)?;
                        }
                        Err(e) => {
                            tracing::warn!(code = "get_tx_error", error = ?e, txid = %id);
                            return Err(DomainError::Unavailable);
                        }
                    }
                }
                Ok::<(), DomainError>(())
            })?;
            let mut balances = Vec::new();
            for id in s.db.get_account_ids().map_err(sdk_error)? {
                if let Some(a) = s.db.get_account(id).map_err(sdk_error)?
                    && let Some(room) = a
                        .name()
                        .and_then(|n| n.strip_prefix("zsplit:"))
                        .and_then(|v| uuid::Uuid::parse_str(v).ok())
                {
                    balances.push(s.balance(room)?);
                }
            }
            Ok(balances)
        })
        .await
    }
    async fn balance(&self, room: RoomId) -> Result<BalanceSnapshot, DomainError> {
        self.run(move |s| s.balance(room)).await
    }
    async fn prepare_payout(
        &self,
        command: FundingCommand,
    ) -> Result<PreparedTransaction, DomainError> {
        self.run(move |s| {
            if command.network != s.config.network || command.amount.value() == 0 {
                return Err(DomainError::InvalidInput);
            }
            let mut h = Sha256::new();
            h.update(b"zsplit-payout-intent-v1\0");
            h.update(command.room_id.as_bytes());
            h.update(command.network.to_string());
            h.update(command.amount.value().to_le_bytes());
            h.update(command.ephemeral_receiver.as_bytes());
            let fingerprint = h.finalize().to_vec();
            let existing: Option<Vec<u8>> = s
                .journal
                .query_row(
                    "SELECT fingerprint FROM intents WHERE id=?1",
                    [command.payout_id.to_string()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage_error)?;
            if let Some(existing) = existing {
                if existing != fingerprint {
                    return Err(DomainError::Conflict);
                }
                return s
                    .prepared(command.payout_id)?
                    .ok_or(DomainError::ReconciliationRequired);
            }
            let id = s.account(command.room_id)?;
            let balance = s.balance(command.room_id)?;
            if balance.chain_height.saturating_sub(balance.scanned_height) > 1000 {
                return Err(DomainError::Unavailable);
            }
            if balance.spendable_zat < command.amount.value() {
                return Err(DomainError::InsufficientFunds);
            }
            let decoded = Address::decode(
                &consensus::Network::TestNetwork,
                &command.ephemeral_receiver,
            )
            .ok_or(DomainError::InvalidInput)?;
            let Address::Unified(ua) = decoded else {
                return Err(DomainError::InvalidInput);
            };
            // Force Orchard even when the client supplies a UA with multiple receivers.
            if ua
                .expiry_height()
                .is_some_and(|h| u32::from(h) <= balance.chain_height)
                || ua.expiry_time().is_some_and(|t| {
                    t <= std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(sdk_error)
                        .map(|d| d.as_secs())
                        .unwrap_or(u64::MAX)
                })
            {
                return Err(DomainError::InvalidInput);
            }
            let orchard = ua.orchard().copied().ok_or(DomainError::InvalidInput)?;
            let receiver = Address::Unified(Box::new(
                UnifiedAddress::from_receivers(Some(orchard), None, None, None, None)
                    .ok_or(DomainError::InvalidInput)?,
            ));
            let (spend, output) = match (
                &s.config.sapling_spend_params,
                &s.config.sapling_output_params,
            ) {
                (Some(a), Some(b)) if a.is_file() && b.is_file() => (a, b),
                _ => return Err(DomainError::Unavailable),
            };
            let prover = std::panic::catch_unwind(|| LocalTxProver::new(spend, output))
                .map_err(|_| DomainError::Unavailable)?;
            let proposal = wallet::propose_standard_transfer_to_address::<
                _,
                _,
                <Db as zcash_client_backend::data_api::WalletCommitmentTrees>::Error,
            >(
                &mut s.db,
                &consensus::Network::TestNetwork,
                StandardFeeRule::Zip317,
                id,
                policy(s.config.confirmations)?,
                &receiver,
                Zatoshis::from_u64(command.amount.value()).map_err(sdk_error)?,
                None,
                None,
                active_output_pool(BlockHeight::from_u32(
                    balance.chain_height.saturating_add(1),
                )),
                None,
                None,
            )
            .map_err(sdk_error)?;
            if proposal.steps().len() != 1
                || u64::from(proposal.steps().first().balance().fee_required())
                    > zsplit_domain::DEFAULT_FEE_RESERVE
            {
                return Err(DomainError::InvalidInput);
            }
            let account =
                s.db.get_account(id)
                    .map_err(sdk_error)?
                    .ok_or(DomainError::NotFound)?;
            let account_index = account
                .source()
                .key_derivation()
                .ok_or(DomainError::InvalidInput)?
                .account_index();
            let usk = UnifiedSpendingKey::from_seed(
                &consensus::Network::TestNetwork,
                s.seed.expose_secret(),
                account_index,
            )
            .map_err(sdk_error)?;
            // This FULL-synchronous marker precedes any operation that can store a signed tx.
            s.journal
                .execute(
                    "INSERT INTO intents(id,fingerprint,state) VALUES(?1,?2,'preparing')",
                    params![command.payout_id.to_string(), fingerprint],
                )
                .map_err(storage_error)?;
            let mut rng = rand_chacha::ChaCha20Rng::try_from_rng(&mut rand10::rngs::SysRng)
                .map_err(sdk_error)?;
            let ids = wallet::create_proposed_transactions::<
                _,
                _,
                std::convert::Infallible,
                _,
                std::convert::Infallible,
                _,
            >(
                &mut s.db,
                &consensus::Network::TestNetwork,
                &SystemClock,
                &mut rng,
                &prover,
                &prover,
                &SpendingKeys::from_unified_spending_key(usk),
                OvkPolicy::Sender,
                &proposal,
                None,
            )
            .map_err(|_| DomainError::ReconciliationRequired)?;
            let txid = *ids.first();
            let tx =
                s.db.get_transaction(txid)
                    .map_err(sdk_error)?
                    .ok_or(DomainError::ReconciliationRequired)?;
            let mut raw_transaction = Vec::new();
            tx.write(&mut raw_transaction).map_err(sdk_error)?;
            s.journal
                .execute(
                    "UPDATE intents SET state='prepared',txid=?2,raw=?3 WHERE id=?1",
                    params![
                        command.payout_id.to_string(),
                        txid.to_string(),
                        raw_transaction
                    ],
                )
                .map_err(storage_error)?;
            Ok(PreparedTransaction {
                payout_id: command.payout_id,
                txid: txid.to_string(),
                raw_transaction,
            })
        })
        .await
    }
    async fn broadcast(&self, tx: &PreparedTransaction) -> Result<(), DomainError> {
        let expected = tx.clone();
        let mut client = self.client.clone();
        let handle = tokio::runtime::Handle::current();
        self.run(move |s| {
            let actual = s
                .prepared(expected.payout_id)?
                .ok_or(DomainError::ReconciliationRequired)?;
            if actual.txid != expected.txid || actual.raw_transaction != expected.raw_transaction {
                return Err(DomainError::Conflict);
            }
            let result = handle
                .block_on(client.send_transaction(RawTransaction {
                    data: actual.raw_transaction,
                    height: 0,
                }))
                .map_err(|_| DomainError::ReconciliationRequired)?
                .into_inner();
            if result.error_code != 0 {
                return Err(DomainError::ReconciliationRequired);
            }
            Ok(())
        })
        .await
    }
    async fn transaction_status(&self, txid: &str) -> Result<ChainTxStatus, DomainError> {
        let id = TxId::from_hex(txid).ok_or(DomainError::InvalidInput)?;
        let mut client = self.client.clone();
        let handle = tokio::runtime::Handle::current();
        self.run(move |s| {
            let previous: Option<u32> = s.journal.query_row("SELECT mined_height FROM chain_observations WHERE txid=?1", [id.to_string()], |r| r.get(0)).optional().map_err(storage_error)?.flatten();
            let raw = match handle.block_on(client.get_transaction(TxFilter { block: None, index: 0, hash: id.as_ref().to_vec() })) {
                Ok(r) => r.into_inner(),
                Err(e) if e.code() == tonic::Code::NotFound => return Ok(if previous.is_some() { ChainTxStatus::Reorged } else { ChainTxStatus::Unknown }),
                Err(_) => return Err(DomainError::Unavailable),
            };
            if raw.height == u64::MAX { return Ok(ChainTxStatus::Reorged); }
            if raw.height == 0 { return Ok(if previous.is_some() { ChainTxStatus::Reorged } else { ChainTxStatus::Mempool }); }
            let height = u32::try_from(raw.height).map_err(sdk_error)?;
            // A gRPC success alone is never confirmation: native block scanning must agree.
            if s.db.get_tx_height(id).map_err(sdk_error)? != Some(BlockHeight::from_u32(height)) { return Ok(ChainTxStatus::Unknown); }
            let summary = s.db.get_wallet_summary(policy(s.config.confirmations)?).map_err(sdk_error)?.ok_or(DomainError::Unavailable)?;
            if !summary.is_synced() { return Ok(ChainTxStatus::Unknown); }
            let tip: u32 = summary.chain_tip_height().into(); if height > tip { return Ok(ChainTxStatus::Unknown); }
            s.journal.execute("INSERT INTO chain_observations(txid,mined_height) VALUES(?1,?2) ON CONFLICT(txid) DO UPDATE SET mined_height=excluded.mined_height", params![id.to_string(),height]).map_err(storage_error)?;
            Ok(ChainTxStatus::Mined { height, confirmations: tip - height + 1 })
        }).await
    }
    async fn recover_prepared(
        &self,
        payout: uuid::Uuid,
    ) -> Result<Option<PreparedTransaction>, DomainError> {
        self.run(move |s| s.prepared(payout)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(root: &Path) -> WalletConfig {
        WalletConfig {
            data_dir: root.join("wallet"),
            seed_file: root.join("seed"),
            lightwalletd_url: "https://testnet.zec.rocks:443".into(),
            network: zsplit_domain::Network::Testnet,
            birthday_height: None,
            confirmations: 10,
            sapling_spend_params: None,
            sapling_output_params: None,
        }
    }
    #[test]
    fn initialize_official_wallet_and_persist_intent_quarantine() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        NativeWallet::create_seed_file(&config.seed_file).unwrap();
        assert!(NativeWallet::create_seed_file(&config.seed_file).is_err());
        let (state, lock) = NativeWallet::initialize(config.clone()).unwrap();
        assert!(state.db.get_account_ids().unwrap().is_empty());
        assert!(NativeWallet::initialize(config.clone()).is_err());
        let id = uuid::Uuid::new_v4();
        state
            .journal
            .execute(
                "INSERT INTO intents(id,fingerprint,state) VALUES(?1,?2,'preparing')",
                params![id.to_string(), vec![1u8; 32]],
            )
            .unwrap();
        assert!(matches!(
            state.prepared(id),
            Err(DomainError::ReconciliationRequired)
        ));
        assert!(state.prepared(uuid::Uuid::new_v4()).unwrap().is_none());
        drop(state);
        drop(lock);
        let (state, _) = NativeWallet::initialize(config).unwrap();
        assert!(matches!(
            state.prepared(id),
            Err(DomainError::ReconciliationRequired)
        ));
    }
    #[test]
    fn native_account_derivation_and_seed_mismatch() {
        use zcash_client_backend::data_api::chain::ChainState;
        use zcash_protocol::consensus::Parameters;
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        NativeWallet::create_seed_file(&config.seed_file).unwrap();
        let (mut state, lock) = NativeWallet::initialize(config.clone()).unwrap();
        let height = consensus::Network::TestNetwork
            .activation_height(consensus::NetworkUpgrade::Nu5)
            .unwrap();
        let birthday = AccountBirthday::from_parts(
            ChainState::empty(height - 1, zcash_primitives::block::BlockHash([0; 32])),
            None,
        );
        let room = uuid::Uuid::new_v4();
        let (id, _) = state
            .db
            .create_account(&format!("zsplit:{room}"), &state.seed, &birthday, None)
            .unwrap();
        assert_eq!(state.account(room).unwrap(), id);
        let a = state.db.get_account(id).unwrap().unwrap();
        let (address, _) = a
            .uivk()
            .find_address(
                zip32::DiversifierIndex::from(0u32),
                UnifiedAddressRequest::ORCHARD,
            )
            .unwrap();
        assert!(address.orchard().is_some());
        assert!(address.sapling().is_none());
        drop(state);
        drop(lock);
        let original = load_seed(&config.seed_file).unwrap();
        fs::remove_file(&config.seed_file).unwrap();
        NativeWallet::create_seed_file(&config.seed_file).unwrap();
        assert_ne!(
            original.expose_secret(),
            load_seed(&config.seed_file).unwrap().expose_secret()
        );
        assert!(NativeWallet::initialize(config).is_err());
    }
    #[test]
    fn current_testnet_uses_ironwood_and_checks_consensus_branch() {
        let activation = consensus::Network::TestNetwork
            .activation_height(consensus::NetworkUpgrade::Nu6_3)
            .unwrap();
        assert_eq!(active_output_pool(activation - 1), ShieldedPool::Orchard);
        assert_eq!(active_output_pool(activation), ShieldedPool::Ironwood);
        let mut info = zcash_client_backend::proto::service::LightdInfo {
            chain_name: "test".into(),
            block_height: 4_581_454,
            consensus_branch_id: "77190ad9".into(),
            ..Default::default()
        };
        assert!(validate_chain_info(&info).is_ok());
        info.consensus_branch_id = "ffffffff".into();
        assert!(matches!(
            validate_chain_info(&info),
            Err(DomainError::NetworkMismatch)
        ));
        info.chain_name = "main".into();
        assert!(matches!(
            validate_chain_info(&info),
            Err(DomainError::NetworkMismatch)
        ));
    }
    #[test]
    fn rejects_unsafe_seed_permissions_and_unsupported_networks() {
        let temp = tempfile::tempdir().unwrap();
        let mut cfg = config(temp.path());
        cfg.network = zsplit_domain::Network::Mainnet;
        assert!(cfg.validate().is_err());
        cfg.network = zsplit_domain::Network::Regtest;
        assert!(cfg.validate().is_err());
        cfg.network = zsplit_domain::Network::Testnet;
        cfg.confirmations = 0;
        assert!(cfg.validate().is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            NativeWallet::create_seed_file(&cfg.seed_file).unwrap();
            fs::set_permissions(&cfg.seed_file, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(load_seed(&cfg.seed_file).is_err());
        }
    }
}
