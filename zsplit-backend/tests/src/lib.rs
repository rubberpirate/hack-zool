//! Integration fixtures only. Synthetic balances never enter the deployed application.
use chrono::{Duration, Utc};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use zsplit_claims::{ReceiverCipher, TokenService};
use zsplit_domain::{BalanceSnapshot, Network};
use zsplit_store::{NewClaim, NewRoom, RedeemCommand, Store};

pub struct Fixture {
    pub store: Store,
    pub owner: Uuid,
    pub room: Uuid,
    pub claims: Vec<NewClaim>,
    pub cipher: ReceiverCipher,
    admin: PgPool,
    schema: String,
}
impl Fixture {
    pub async fn new(count: usize) -> Self {
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL is required for real PostgreSQL tests");
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("zsplit_test_{}", Uuid::new_v4().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .after_connect(move |connection, _| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(&search_path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        let store = Store::from_pool(pool);
        store.migrate().await.unwrap();
        let owner = Uuid::new_v4();
        let room = Uuid::new_v4();
        store
            .ensure_organizer(owner, &format!("test-{owner}"))
            .await
            .unwrap();
        store
            .create_room(NewRoom {
                id: room,
                organizer_id: owner,
                network: Network::Testnet,
                deposit_address: "integration-fixture-not-a-chain-address".into(),
                birthday_height: 100,
                amounts: vec![100_000; count],
                fee_reserve_zat: 100_000,
                expires_at: Utc::now() + Duration::hours(1),
            })
            .await
            .unwrap();
        // Explicit synthetic balance fixture for accounting tests; never a blockchain success assertion.
        store
            .update_balance(BalanceSnapshot {
                room_id: room,
                confirmed_zat: 100_000_000,
                spendable_zat: 100_000_000,
                chain_height: 100,
                scanned_height: 100,
            })
            .await
            .unwrap();
        let tokens = TokenService::new([71; 32]);
        let claims = (0..count)
            .map(|_| NewClaim {
                id: Uuid::new_v4(),
                amount_zat: 100_000,
                token_digest: tokens.issue().digest,
            })
            .collect();
        Self {
            store,
            owner,
            room,
            claims,
            cipher: ReceiverCipher::new([72; 32]),
            admin,
            schema,
        }
    }
    pub async fn issue(&self) {
        self.store
            .issue_claims(self.room, self.owner, &self.claims)
            .await
            .unwrap();
    }
    pub fn redeem(&self, index: usize, key: Uuid) -> RedeemCommand {
        let claim = &self.claims[index];
        RedeemCommand {
            claim_id: claim.id,
            token_digest: claim.token_digest,
            network: Network::Testnet,
            idempotency_key: key,
            encrypted_receiver: self
                .cipher
                .encrypt(claim.id, "test-engine-ephemeral-receiver")
                .unwrap(),
            request_fingerprint: vec![index as u8; 32],
        }
    }
    pub async fn finish(self) {
        self.store.pool().close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
