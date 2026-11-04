//! PostgreSQL metadata and payout intent storage. Wallet balances enter only through the native worker.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;
use zsplit_domain::{
    BalanceSnapshot, ChainTxStatus, DomainError, MAX_TICKETS, MAX_ZATS, Network,
    PreparedTransaction,
};

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Room {
    pub id: Uuid,
    pub organizer_id: Uuid,
    pub network: String,
    pub state: String,
    pub deposit_address: String,
    pub birthday_height: i64,
    pub payout_amounts: sqlx::types::Json<Vec<u64>>,
    pub fee_reserve_zat: i64,
    pub confirmed_zat: i64,
    pub spendable_zat: i64,
    pub reserved_zat: i64,
    pub chain_height: i64,
    pub scanned_height: i64,
    pub balance_updated_at: Option<DateTime<Utc>>,
    pub claims_issued: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
#[derive(Clone)]
pub struct NewRoom {
    pub id: Uuid,
    pub organizer_id: Uuid,
    pub network: Network,
    pub deposit_address: String,
    pub birthday_height: u32,
    pub amounts: Vec<u64>,
    pub fee_reserve_zat: u64,
    pub expires_at: DateTime<Utc>,
}
#[derive(Clone)]
pub struct NewClaim {
    pub id: Uuid,
    pub amount_zat: u64,
    pub token_digest: [u8; 32],
}
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClaimRecord {
    pub id: Uuid,
    pub room_id: Uuid,
    pub amount_zat: i64,
    pub state: String,
    pub expires_at: DateTime<Utc>,
    pub payout_id: Option<Uuid>,
    pub sweep_reported: bool,
    pub network: String,
    pub txid: Option<String>,
    pub confirmations: i32,
}
#[derive(Clone)]
pub struct RedeemCommand {
    pub claim_id: Uuid,
    pub token_digest: [u8; 32],
    pub network: Network,
    pub idempotency_key: Uuid,
    pub encrypted_receiver: Vec<u8>,
    pub request_fingerprint: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Redemption {
    pub claim_id: Uuid,
    pub payout_id: Uuid,
    pub status: String,
}
#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PayoutJob {
    pub id: Uuid,
    pub claim_id: Uuid,
    pub room_id: Uuid,
    pub amount_zat: i64,
    pub encrypted_receiver: Vec<u8>,
    pub txid: Option<String>,
    pub raw_transaction: Option<Vec<u8>>,
    pub state: String,
    pub network: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoomSummary {
    pub room_id: Uuid,
    pub confirmed_zat: i64,
    pub reserved_zat: i64,
    pub spendable_zat: i64,
    pub reserved_count: i64,
    pub pending_count: i64,
    pub funded_confirmed_count: i64,
    pub expired_count: i64,
    pub sweep_reported_count: i64,
    pub manual_review_count: i64,
}
fn db(_: sqlx::Error) -> DomainError {
    DomainError::Storage
}
fn amount(v: u64) -> Result<i64, DomainError> {
    if v == 0 || v > MAX_ZATS {
        Err(DomainError::InvalidInput)
    } else {
        Ok(v as i64)
    }
}
fn fresh(room: &Room) -> Result<(), DomainError> {
    if let Some(at) = room.balance_updated_at {
        if Utc::now().signed_duration_since(at).num_seconds() > 6 * 3600 {
            return Err(DomainError::Unavailable);
        }
    }
    if room.chain_height.saturating_sub(room.scanned_height) > 1000 {
        return Err(DomainError::Unavailable);
    }
    Ok(())
}

impl Store {
    pub async fn connect(url: &str) -> Result<Self, DomainError> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(url)
            .await
            .map_err(db)?;
        Ok(Self { pool })
    }
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    pub async fn migrate(&self) -> Result<(), DomainError> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("MIGRATION ERROR: {:?}", e);
                DomainError::Storage
            })
    }
    pub async fn ready(&self) -> Result<(), DomainError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(())
    }
    pub async fn ensure_organizer(&self, id: Uuid, subject: &str) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO organizers(id,auth_subject) VALUES($1,$2) ON CONFLICT(id) DO NOTHING",
        )
        .bind(id)
        .bind(subject)
        .execute(&self.pool)
        .await
        .map_err(db)?;
        Ok(())
    }
    pub async fn create_room(&self, cmd: NewRoom) -> Result<Room, DomainError> {
        validate_amounts(&cmd.amounts, cmd.fee_reserve_zat)?;
        if cmd.expires_at <= Utc::now() || cmd.expires_at > Utc::now() + chrono::Duration::days(30)
        {
            return Err(DomainError::InvalidInput);
        }
        sqlx::query_as("INSERT INTO rooms(id,organizer_id,network,deposit_address,birthday_height,payout_amounts,fee_reserve_zat,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *")
   .bind(cmd.id).bind(cmd.organizer_id).bind(cmd.network.to_string()).bind(cmd.deposit_address).bind(i64::from(cmd.birthday_height)).bind(sqlx::types::Json(cmd.amounts)).bind(amount(cmd.fee_reserve_zat)?).bind(cmd.expires_at).fetch_one(&self.pool).await.map_err(db)
    }
    pub async fn room(&self, id: Uuid, owner: Uuid) -> Result<Room, DomainError> {
        sqlx::query_as("SELECT * FROM rooms WHERE id=$1 AND organizer_id=$2")
            .bind(id)
            .bind(owner)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?
            .ok_or(DomainError::NotFound)
    }
    pub async fn rooms(&self) -> Result<Vec<Room>, DomainError> {
        sqlx::query_as("SELECT * FROM rooms ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(db)
    }
    pub async fn issue_claims(
        &self,
        room_id: Uuid,
        owner: Uuid,
        claims: &[NewClaim],
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let room: Room =
            sqlx::query_as("SELECT * FROM rooms WHERE id=$1 AND organizer_id=$2 FOR UPDATE")
                .bind(room_id)
                .bind(owner)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or(DomainError::NotFound)?;
        fresh(&room)?;
        if room.claims_issued || room.expires_at <= Utc::now() || room.state != "ready" {
            return Err(DomainError::Conflict);
        }
        if claims.len() != room.payout_amounts.len()
            || claims
                .iter()
                .zip(room.payout_amounts.iter())
                .any(|(c, a)| c.amount_zat != *a)
        {
            return Err(DomainError::InvalidInput);
        }
        let reserve = validate_amounts(&room.payout_amounts, room.fee_reserve_zat as u64)?;
        if room.spendable_zat - room.reserved_zat < reserve {
            return Err(DomainError::InsufficientFunds);
        }
        // Bound arrays reduce 100 round trips for 50 tickets to two INSERTs without changing atomicity.
        let ids: Vec<Uuid> = claims.iter().map(|c| c.id).collect();
        let values: Vec<i64> = claims
            .iter()
            .map(|c| amount(c.amount_zat))
            .collect::<Result<_, _>>()?;
        let digests: Vec<Vec<u8>> = claims.iter().map(|c| c.token_digest.to_vec()).collect();
        sqlx::query("INSERT INTO claims(id,room_id,amount_zat,fee_budget_zat,token_digest,expires_at) SELECT c.id,$4,c.amount_zat,$5,c.token_digest,$6 FROM UNNEST($1::uuid[],$2::bigint[],$3::bytea[]) AS c(id,amount_zat,token_digest)")
          .bind(&ids).bind(&values).bind(&digests).bind(room_id).bind(room.fee_reserve_zat).bind(room.expires_at).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO ledger_entries(room_id,claim_id,kind,amount_zat) SELECT $1,c.id,'reserve',c.amount_zat+$4 FROM UNNEST($2::uuid[],$3::bigint[]) AS c(id,amount_zat)")
          .bind(room_id).bind(&ids).bind(&values).bind(room.fee_reserve_zat).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE rooms SET claims_issued=true,reserved_zat=reserved_zat+$2 WHERE id=$1")
            .bind(room_id)
            .bind(reserve)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        audit(&mut tx, room_id, None, "claims_issued", "organizer").await?;
        tx.commit().await.map_err(db)
    }
    pub async fn claim(&self, id: Uuid, digest: &[u8]) -> Result<ClaimRecord, DomainError> {
        sqlx::query_as("SELECT c.id,c.room_id,c.amount_zat,c.state,c.expires_at,c.payout_id,c.sweep_reported,r.network,j.txid,COALESCE(t.confirmations,0) confirmations FROM claims c JOIN rooms r ON r.id=c.room_id LEFT JOIN payout_jobs j ON j.claim_id=c.id LEFT JOIN wallet_transactions t ON t.payout_id=j.id WHERE c.id=$1 AND c.token_digest=$2 AND c.kind='claim'").bind(id).bind(digest).fetch_optional(&self.pool).await.map_err(db)?.ok_or(DomainError::NotFound)
    }
    pub async fn enqueue_redemption(&self, cmd: RedeemCommand) -> Result<Redemption, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        // Lock room before claim, matching expiry/issuance and avoiding lock-order inversions.
        let room:Room=sqlx::query_as("SELECT r.* FROM rooms r JOIN claims c ON c.room_id=r.id WHERE c.id=$1 AND c.token_digest=$2 AND c.kind='claim' FOR UPDATE OF r").bind(cmd.claim_id).bind(cmd.token_digest.as_slice()).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(DomainError::NotFound)?;
        let row = sqlx::query("SELECT state,expires_at FROM claims WHERE id=$1 FOR UPDATE")
            .bind(cmd.claim_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(job) = sqlx::query(
            "SELECT id,idempotency_key,request_fingerprint FROM payout_jobs WHERE claim_id=$1",
        )
        .bind(cmd.claim_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        {
            if job.get::<Uuid, _>("idempotency_key") != cmd.idempotency_key
                || job.get::<Vec<u8>, _>("request_fingerprint") != cmd.request_fingerprint
            {
                return Err(DomainError::Conflict);
            }
            return Ok(Redemption {
                claim_id: cmd.claim_id,
                payout_id: job.get("id"),
                status: "funding_queued".into(),
            });
        }
        if room.network != cmd.network.to_string() {
            return Err(DomainError::NetworkMismatch);
        }
        fresh(&room)?;
        if row.get::<String, _>("state") != "reserved" || room.state != "ready" {
            return Err(DomainError::Conflict);
        }
        if row.get::<DateTime<Utc>, _>("expires_at") <= Utc::now() {
            return Err(DomainError::Expired);
        }
        if cmd.encrypted_receiver.is_empty() || cmd.request_fingerprint.len() != 32 {
            return Err(DomainError::InvalidInput);
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO payout_jobs(id,claim_id,room_id,idempotency_key,encrypted_ephemeral_receiver,request_fingerprint) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(cmd.claim_id).bind(room.id).bind(cmd.idempotency_key).bind(cmd.encrypted_receiver).bind(cmd.request_fingerprint).execute(&mut *tx).await.map_err(db)?;
        sqlx::query(
            "UPDATE claims SET state='funding_queued',payout_id=$2,version=version+1 WHERE id=$1",
        )
        .bind(cmd.claim_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        ledger(&mut tx, room.id, Some(cmd.claim_id), "queue", 0).await?;
        audit(
            &mut tx,
            room.id,
            Some(cmd.claim_id),
            "funding_queued",
            "bearer",
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(Redemption {
            claim_id: cmd.claim_id,
            payout_id: id,
            status: "funding_queued".into(),
        })
    }
    pub async fn report_sweep(&self, id: Uuid, digest: &[u8]) -> Result<(), DomainError> {
        let n=sqlx::query("UPDATE claims SET sweep_reported=true,version=version+1 WHERE id=$1 AND token_digest=$2 AND kind='claim' AND state='funded_confirmed'").bind(id).bind(digest).execute(&self.pool).await.map_err(db)?.rows_affected();
        if n == 0 {
            return Err(DomainError::Conflict);
        }
        Ok(())
    }
    pub async fn summary(&self, id: Uuid, owner: Uuid) -> Result<RoomSummary, DomainError> {
        sqlx::query_as("SELECT r.id room_id,r.confirmed_zat,r.reserved_zat,r.spendable_zat,count(c.id) FILTER(WHERE c.state='reserved') reserved_count,count(c.id) FILTER(WHERE c.state IN ('funding_queued','preparing','funding_broadcast','funded_unconfirmed','reorged','reconciling','ambiguous_broadcast')) pending_count,count(c.id) FILTER(WHERE c.state='funded_confirmed') funded_confirmed_count,count(c.id) FILTER(WHERE c.state='expired') expired_count,count(c.id) FILTER(WHERE c.sweep_reported) sweep_reported_count,count(c.id) FILTER(WHERE c.state='manual_review') manual_review_count FROM rooms r LEFT JOIN claims c ON c.room_id=r.id AND c.kind='claim' WHERE r.id=$1 AND r.organizer_id=$2 GROUP BY r.id").bind(id).bind(owner).fetch_optional(&self.pool).await.map_err(db)?.ok_or(DomainError::NotFound)
    }
    pub async fn expire_claims(&self) -> Result<u64, DomainError> {
        let mut total = 0;
        // One room per transaction keeps issue/redeem/expiry/withdrawal accounting serialized.
        for room in self.rooms().await? {
            let mut tx = self.pool.begin().await.map_err(db)?;
            sqlx::query("SELECT id FROM rooms WHERE id=$1 FOR UPDATE")
                .bind(room.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let rows=sqlx::query("UPDATE claims SET state='expired',reservation_released=true,version=version+1 WHERE room_id=$1 AND state='reserved' AND expires_at<=now() RETURNING id,amount_zat+fee_budget_zat reserve").bind(room.id).fetch_all(&mut *tx).await.map_err(db)?;
            let mut released = 0_i64;
            for r in &rows {
                let v: i64 = r.get("reserve");
                released += v;
                ledger(&mut tx, room.id, Some(r.get("id")), "release_expiry", v).await?;
            }
            if released > 0 {
                sqlx::query("UPDATE rooms SET reserved_zat=reserved_zat-$2 WHERE id=$1")
                    .bind(room.id)
                    .bind(released)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
            tx.commit().await.map_err(db)?;
            total += rows.len() as u64;
        }
        Ok(total)
    }
    pub async fn enqueue_withdrawal(
        &self,
        room_id: Uuid,
        owner: Uuid,
        amount_zat: u64,
        idempotency_key: Uuid,
        encrypted_receiver: Vec<u8>,
        fingerprint: Vec<u8>,
    ) -> Result<Redemption, DomainError> {
        let value = amount(amount_zat)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let room: Room =
            sqlx::query_as("SELECT * FROM rooms WHERE id=$1 AND organizer_id=$2 FOR UPDATE")
                .bind(room_id)
                .bind(owner)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or(DomainError::NotFound)?;
        if let Some(row)=sqlx::query("SELECT j.id,j.claim_id,j.request_fingerprint FROM payout_jobs j JOIN claims c ON c.id=j.claim_id WHERE j.room_id=$1 AND j.idempotency_key=$2 AND c.kind='withdrawal'").bind(room_id).bind(idempotency_key).fetch_optional(&mut *tx).await.map_err(db)? {
   if row.get::<Vec<u8>,_>("request_fingerprint")!=fingerprint{return Err(DomainError::Conflict)}
   return Ok(Redemption{claim_id:row.get("claim_id"),payout_id:row.get("id"),status:"funding_queued".into()});
  }
        fresh(&room)?;
        if room.state != "ready"
            || room.spendable_zat - room.reserved_zat < value + room.fee_reserve_zat
        {
            return Err(DomainError::InsufficientFunds);
        }
        if encrypted_receiver.is_empty() || fingerprint.len() != 32 {
            return Err(DomainError::InvalidInput);
        }
        // Cipher AAD for withdrawals is the public idempotency key; it is also the internal claim ID.
        let claim_id = idempotency_key;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO claims(id,room_id,kind,amount_zat,fee_budget_zat,state,expires_at,payout_id) VALUES($1,$2,'withdrawal',$3,$4,'funding_queued',now()+interval '30 days',$5)").bind(claim_id).bind(room_id).bind(value).bind(room.fee_reserve_zat).bind(id).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO payout_jobs(id,claim_id,room_id,idempotency_key,encrypted_ephemeral_receiver,request_fingerprint) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(claim_id).bind(room_id).bind(idempotency_key).bind(encrypted_receiver).bind(fingerprint).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE rooms SET reserved_zat=reserved_zat+$2 WHERE id=$1")
            .bind(room_id)
            .bind(value + room.fee_reserve_zat)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        ledger(
            &mut tx,
            room_id,
            Some(claim_id),
            "withdraw_reserve",
            value + room.fee_reserve_zat,
        )
        .await?;
        audit(
            &mut tx,
            room_id,
            Some(claim_id),
            "withdrawal_queued",
            "organizer",
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(Redemption {
            claim_id,
            payout_id: id,
            status: "funding_queued".into(),
        })
    }
    pub async fn lease_job(&self, worker: Uuid) -> Result<Option<PayoutJob>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let id:Option<Uuid>=sqlx::query_scalar("SELECT id FROM payout_jobs WHERE state IN ('queued','prepared') AND next_attempt_at<=now() AND (lease_until IS NULL OR lease_until<now()) ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(db)?;
        let Some(id) = id else { return Ok(None) };
        sqlx::query("UPDATE payout_jobs SET state=CASE WHEN raw_transaction IS NULL THEN 'preparing' ELSE 'prepared' END,lease_owner=$2,lease_until=now()+interval '30 minutes',attempts=attempts+1,updated_at=now() WHERE id=$1").bind(id).bind(worker).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE claims SET state='preparing',version=version+1 WHERE payout_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let job=sqlx::query_as("SELECT j.id,j.claim_id,j.room_id,c.amount_zat,COALESCE(j.encrypted_ephemeral_receiver,''::bytea) encrypted_receiver,j.txid,j.raw_transaction,j.state,r.network FROM payout_jobs j JOIN claims c ON c.id=j.claim_id JOIN rooms r ON r.id=j.room_id WHERE j.id=$1").bind(id).fetch_one(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(Some(job))
    }
    pub async fn recover_stale_jobs(&self) -> Result<u64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let rows=sqlx::query("UPDATE payout_jobs SET state=CASE WHEN raw_transaction IS NULL THEN 'manual_review' ELSE 'prepared' END,lease_owner=NULL,lease_until=NULL,error_code='interrupted_worker',updated_at=now() WHERE state IN ('preparing','prepared') AND lease_until<now() RETURNING claim_id,state").fetch_all(&mut *tx).await.map_err(db)?;
        for row in &rows {
            let state: &str = if row.get::<String, _>("state") == "manual_review" {
                "manual_review"
            } else {
                "preparing"
            };
            sqlx::query("UPDATE claims SET state=$2,version=version+1 WHERE id=$1")
                .bind(row.get::<Uuid, _>("claim_id"))
                .bind(state)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(rows.len() as u64)
    }
    pub async fn save_prepared(
        &self,
        id: Uuid,
        worker: Uuid,
        prepared: &PreparedTransaction,
    ) -> Result<(), DomainError> {
        if prepared.payout_id != id
            || prepared.txid.len() != 64
            || !prepared.txid.bytes().all(|b| b.is_ascii_hexdigit())
            || prepared.raw_transaction.is_empty()
        {
            return Err(DomainError::InvalidInput);
        }
        let n=sqlx::query("UPDATE payout_jobs SET state='prepared',txid=$3,raw_transaction=$4,encrypted_ephemeral_receiver=NULL,updated_at=now() WHERE id=$1 AND lease_owner=$2 AND state IN ('preparing','prepared') AND (txid IS NULL OR txid=$3)").bind(id).bind(worker).bind(&prepared.txid).bind(&prepared.raw_transaction).execute(&self.pool).await.map_err(db)?.rows_affected();
        if n == 0 {
            return Err(DomainError::Conflict);
        }
        Ok(())
    }
    pub async fn mark_broadcast(&self, id: Uuid, worker: Uuid) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let row=sqlx::query("UPDATE payout_jobs SET state='funding_broadcast',lease_owner=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND lease_owner=$2 AND txid IS NOT NULL RETURNING txid,room_id,claim_id").bind(id).bind(worker).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(DomainError::Conflict)?;
        sqlx::query(
            "UPDATE claims SET state='funding_broadcast',version=version+1 WHERE payout_id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("INSERT INTO wallet_transactions(txid,payout_id,network,state) SELECT $2,$1,r.network,'funding_broadcast' FROM rooms r WHERE r.id=$3 ON CONFLICT(txid) DO NOTHING").bind(id).bind(row.get::<String,_>("txid")).bind(row.get::<Uuid,_>("room_id")).execute(&mut *tx).await.map_err(db)?;
        audit(
            &mut tx,
            row.get("room_id"),
            Some(row.get("claim_id")),
            "funding_broadcast",
            "worker",
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    pub async fn mark_manual_review(&self, id: Uuid, worker: Uuid) -> Result<(), DomainError> {
        self.worker_state(id, worker, "manual_review").await
    }
    pub async fn defer_job(&self, id: Uuid, worker: Uuid) -> Result<(), DomainError> {
        // Only serialized transactions are retried. A failed construction is ambiguous and quarantined.
        let mut tx = self.pool.begin().await.map_err(db)?;
        let row=sqlx::query("UPDATE payout_jobs SET state=CASE WHEN raw_transaction IS NULL THEN 'manual_review' ELSE 'prepared' END,next_attempt_at=now()+make_interval(secs=>LEAST(300,power(2,LEAST(attempts,8))::int)+(abs(hashtext(id::text))%7)),lease_owner=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND lease_owner=$2 RETURNING claim_id,state").bind(id).bind(worker).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(DomainError::Conflict)?;
        if row.get::<String, _>("state") == "manual_review" {
            sqlx::query("UPDATE claims SET state='manual_review',version=version+1 WHERE id=$1")
                .bind(row.get::<Uuid, _>("claim_id"))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)
    }
    async fn worker_state(&self, id: Uuid, worker: Uuid, state: &str) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let claim:Uuid=sqlx::query_scalar("UPDATE payout_jobs SET state=$3,lease_owner=NULL,lease_until=NULL,updated_at=now() WHERE id=$1 AND lease_owner=$2 RETURNING claim_id").bind(id).bind(worker).bind(state).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(DomainError::Conflict)?;
        sqlx::query("UPDATE claims SET state=$2,version=version+1 WHERE id=$1")
            .bind(claim)
            .bind(state)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)
    }
    pub async fn tracked_jobs(&self) -> Result<Vec<PayoutJob>, DomainError> {
        sqlx::query_as("SELECT j.id,j.claim_id,j.room_id,c.amount_zat,COALESCE(j.encrypted_ephemeral_receiver,''::bytea) encrypted_receiver,j.txid,j.raw_transaction,j.state,r.network FROM payout_jobs j JOIN claims c ON c.id=j.claim_id JOIN rooms r ON r.id=j.room_id WHERE j.txid IS NOT NULL AND j.state NOT IN ('queued','preparing','prepared','manual_review') ORDER BY j.created_at").fetch_all(&self.pool).await.map_err(db)
    }
    pub async fn reconcile_job(
        &self,
        id: Uuid,
        status: ChainTxStatus,
        required_confirmations: u32,
    ) -> Result<(), DomainError> {
        if required_confirmations == 0 {
            return Err(DomainError::InvalidInput);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SELECT r.id FROM rooms r JOIN payout_jobs j ON j.room_id=r.id WHERE j.id=$1 FOR UPDATE OF r").bind(id).execute(&mut *tx).await.map_err(db)?;
        let row = sqlx::query(
            "SELECT claim_id,room_id,state,txid FROM payout_jobs WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(DomainError::NotFound)?;
        if row.get::<Option<String>, _>("txid").is_none() {
            return Err(DomainError::Conflict);
        }
        let previous: String = row.get("state");
        let (state, height, confirmations) = match status {
            ChainTxStatus::Mined {
                height,
                confirmations,
            } => (
                if confirmations >= required_confirmations {
                    "funded_confirmed"
                } else {
                    "funded_unconfirmed"
                },
                Some(i64::from(height)),
                confirmations as i32,
            ),
            ChainTxStatus::Mempool => ("funding_broadcast", None, 0),
            ChainTxStatus::Unknown | ChainTxStatus::Reorged => (
                if previous == "funded_confirmed" || previous == "funded_unconfirmed" {
                    "reorged"
                } else {
                    "reconciling"
                },
                None,
                0,
            ),
            ChainTxStatus::Expired => ("manual_review", None, 0),
        };
        sqlx::query("UPDATE payout_jobs SET state=$2,updated_at=now() WHERE id=$1")
            .bind(id)
            .bind(state)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE claims SET state=$2,version=version+1 WHERE payout_id=$1")
            .bind(id)
            .bind(state)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE wallet_transactions SET state=$2,block_height=$3,confirmations=$4,updated_at=now() WHERE payout_id=$1").bind(id).bind(state).bind(height).bind(confirmations).execute(&mut *tx).await.map_err(db)?;
        // Any confirmation downgrade invalidates room spending until the next native wallet balance scan.
        if state != "funded_confirmed" && previous == "funded_confirmed" {
            let restored:Option<i64>=sqlx::query_scalar("UPDATE claims SET reservation_released=false WHERE payout_id=$1 AND reservation_released RETURNING amount_zat+fee_budget_zat").bind(id).fetch_optional(&mut *tx).await.map_err(db)?;
            let restored = restored.unwrap_or(0);
            sqlx::query("UPDATE rooms SET state='paused',balance_updated_at=NULL,reserved_zat=reserved_zat+$2 WHERE id=$1").bind(row.get::<Uuid,_>("room_id")).bind(restored).execute(&mut *tx).await.map_err(db)?;
            if restored > 0 {
                ledger(
                    &mut tx,
                    row.get("room_id"),
                    Some(row.get("claim_id")),
                    "reserve_reorg",
                    restored,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(db)
    }
    pub async fn update_balance(&self, snapshot: BalanceSnapshot) -> Result<(), DomainError> {
        if snapshot.confirmed_zat > MAX_ZATS
            || snapshot.spendable_zat > snapshot.confirmed_zat
            || snapshot.scanned_height > snapshot.chain_height
        {
            return Err(DomainError::InvalidInput);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let room: Room = sqlx::query_as("SELECT * FROM rooms WHERE id=$1 FOR UPDATE")
            .bind(snapshot.room_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or(DomainError::NotFound)?;
        // Only release settled reservations when a new SDK snapshot arrives after chain reconciliation.
        let settled=sqlx::query("UPDATE claims c SET reservation_released=true WHERE room_id=$1 AND state='funded_confirmed' AND NOT reservation_released AND EXISTS(SELECT 1 FROM wallet_transactions t WHERE t.payout_id=c.payout_id AND t.updated_at<now() AND t.block_height <= $2) RETURNING id,amount_zat+fee_budget_zat reserve").bind(room.id).bind(i64::from(snapshot.scanned_height)).fetch_all(&mut *tx).await.map_err(db)?;
        let mut released = 0_i64;
        for row in settled {
            let v: i64 = row.get("reserve");
            released += v;
            ledger(
                &mut tx,
                room.id,
                Some(row.get("id")),
                "release_confirmed",
                v,
            )
            .await?;
        }
        let reserve = room.reserved_zat - released;
        let required = if room.claims_issued {
            reserve
        } else {
            validate_amounts(&room.payout_amounts, room.fee_reserve_zat as u64)?
        };
        let max_confirmed = std::cmp::max(room.confirmed_zat, snapshot.confirmed_zat as i64);
        let max_spendable = std::cmp::max(room.spendable_zat, snapshot.spendable_zat as i64);
        let ready = if room.claims_issued || room.state == "ready" {
            true
        } else {
            snapshot
                .chain_height
                .saturating_sub(snapshot.scanned_height)
                <= 1000
                && max_spendable >= required as i64
        };
        sqlx::query("UPDATE rooms SET confirmed_zat=$2,spendable_zat=$3,chain_height=$4,scanned_height=$5,balance_updated_at=now(),reserved_zat=$6,state=CASE WHEN state='closed' THEN state WHEN $7 THEN 'ready' ELSE 'paused' END WHERE id=$1")
            .bind(room.id)
            .bind(max_confirmed)
            .bind(max_spendable)
            .bind(i64::from(snapshot.chain_height))
            .bind(i64::from(snapshot.scanned_height))
            .bind(reserve)
            .bind(ready)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)
    }
    pub async fn dev_fund_room(&self, room_id: Uuid, owner: Uuid) -> Result<(), DomainError> {
        let room = self.room(room_id, owner).await?;
        let required = validate_amounts(&room.payout_amounts, room.fee_reserve_zat as u64)?;
        let height = room.birthday_height as u32;
        self.update_balance(BalanceSnapshot {
            room_id,
            confirmed_zat: required as u64,
            spendable_zat: required as u64,
            chain_height: height + 10,
            scanned_height: height + 10,
        })
        .await
    }
}

pub fn validate_amounts(amounts: &[u64], fee: u64) -> Result<i64, DomainError> {
    if amounts.is_empty() || amounts.len() > MAX_TICKETS || fee > MAX_ZATS {
        return Err(DomainError::InvalidInput);
    }
    let mut total = 0_u64;
    for value in amounts {
        if *value < 10_000 || *value > MAX_ZATS {
            return Err(DomainError::InvalidInput);
        }
        total = total
            .checked_add(*value)
            .and_then(|n| n.checked_add(fee))
            .filter(|n| *n <= MAX_ZATS)
            .ok_or(DomainError::InvalidInput)?;
    }
    Ok(total as i64)
}
async fn ledger(
    tx: &mut Transaction<'_, Postgres>,
    room: Uuid,
    claim: Option<Uuid>,
    kind: &str,
    value: i64,
) -> Result<(), DomainError> {
    sqlx::query("INSERT INTO ledger_entries(room_id,claim_id,kind,amount_zat) VALUES($1,$2,$3,$4)")
        .bind(room)
        .bind(claim)
        .bind(kind)
        .bind(value)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    room: Uuid,
    claim: Option<Uuid>,
    event: &str,
    actor: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO audit_events(room_id,claim_id,event_type,actor_type) VALUES($1,$2,$3,$4)",
    )
    .bind(room)
    .bind(claim)
    .bind(event)
    .bind(actor)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accounting_checks_limits() {
        assert_eq!(validate_amounts(&[10_000; 50], 100_000).unwrap(), 5_500_000);
        assert!(validate_amounts(&[], 1).is_err());
        assert!(validate_amounts(&[MAX_ZATS], 1).is_err());
        assert!(validate_amounts(&[9_999], 1).is_err());
        assert!(validate_amounts(&[10_000; 51], 1).is_err());
    }
}
