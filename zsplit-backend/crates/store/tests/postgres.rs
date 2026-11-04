//! Real PostgreSQL verification. Fixture balances are confined to this test target.
use chrono::{Duration, Utc};
use uuid::Uuid;
use zsplit_domain::{BalanceSnapshot, ChainTxStatus, Network, PreparedTransaction};
use zsplit_store::{NewClaim, NewRoom, RedeemCommand, Store};

async fn fixture() -> (Store, Uuid, Uuid, Uuid, [u8; 32]) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required for PostgreSQL tests");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let room = Uuid::new_v4();
    let claim = Uuid::new_v4();
    store
        .ensure_organizer(owner, &format!("test-{owner}"))
        .await
        .unwrap();
    store
        .create_room(NewRoom {
            id: room,
            organizer_id: owner,
            network: Network::Testnet,
            deposit_address: "test-fixture-only".into(),
            birthday_height: 1,
            amounts: vec![1_000_000],
            fee_reserve_zat: 100_000,
            expires_at: Utc::now() + Duration::hours(1),
        })
        .await
        .unwrap();
    store
        .update_balance(BalanceSnapshot {
            room_id: room,
            confirmed_zat: 3_000_000,
            spendable_zat: 3_000_000,
            chain_height: 100,
            scanned_height: 100,
        })
        .await
        .unwrap();
    let mut digest = [0; 32];
    digest[..16].copy_from_slice(claim.as_bytes());
    digest[16..].copy_from_slice(claim.as_bytes());
    store
        .issue_claims(
            room,
            owner,
            &[NewClaim {
                id: claim,
                amount_zat: 1_000_000,
                token_digest: digest,
            }],
        )
        .await
        .unwrap();
    (store, owner, room, claim, digest)
}
fn command(claim: Uuid, digest: [u8; 32], key: Uuid) -> RedeemCommand {
    RedeemCommand {
        claim_id: claim,
        token_digest: digest,
        network: Network::Testnet,
        idempotency_key: key,
        encrypted_receiver: vec![1; 64],
        request_fingerprint: vec![7; 32],
    }
}

#[tokio::test]
#[ignore = "requires real isolated PostgreSQL DATABASE_URL"]
async fn postgres_one_hundred_racing_redemptions_have_one_intent() {
    let (store, owner, room, claim, digest) = fixture().await;
    let mut racers = tokio::task::JoinSet::new();
    for _ in 0..100 {
        let s = store.clone();
        racers.spawn(async move {
            s.enqueue_redemption(command(claim, digest, Uuid::new_v4()))
                .await
        });
    }
    let mut wins = 0;
    while let Some(result) = racers.join_next().await {
        if result.unwrap().is_ok() {
            wins += 1
        }
    }
    assert_eq!(wins, 1);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM payout_jobs WHERE claim_id=$1")
        .bind(claim)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    let key: Uuid = sqlx::query_scalar("SELECT idempotency_key FROM payout_jobs WHERE claim_id=$1")
        .bind(claim)
        .fetch_one(store.pool())
        .await
        .unwrap();
    let first = store
        .enqueue_redemption(command(claim, digest, key))
        .await
        .unwrap();
    let repeat = store
        .enqueue_redemption(command(claim, digest, key))
        .await
        .unwrap();
    assert_eq!(first.payout_id, repeat.payout_id);
    let mut mismatch = command(claim, digest, key);
    mismatch.request_fingerprint = vec![8; 32];
    assert!(store.enqueue_redemption(mismatch).await.is_err());
    assert_eq!(
        store.summary(room, owner).await.unwrap().reserved_zat,
        1_100_000
    );
    assert!(store.room(room, Uuid::new_v4()).await.is_err());
}

#[tokio::test]
#[ignore = "requires real isolated PostgreSQL DATABASE_URL"]
async fn postgres_crash_quarantine_and_reorg_restore_liability() {
    let (store, owner, room, claim, digest) = fixture().await;
    let op = store
        .enqueue_redemption(command(claim, digest, Uuid::new_v4()))
        .await
        .unwrap();
    sqlx::query("UPDATE payout_jobs SET state='preparing',lease_owner=$2,lease_until=now()-interval '1 minute' WHERE id=$1").bind(op.payout_id).bind(Uuid::new_v4()).execute(store.pool()).await.unwrap();
    store.recover_stale_jobs().await.unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM payout_jobs WHERE id=$1")
        .bind(op.payout_id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(state, "manual_review");
    // Restore a fixture lease and persist test bytes only to exercise SQL state reconciliation.
    let worker = Uuid::new_v4();
    sqlx::query("UPDATE payout_jobs SET state='preparing',lease_owner=$2,lease_until=now()+interval '10 minutes' WHERE id=$1").bind(op.payout_id).bind(worker).execute(store.pool()).await.unwrap();
    store
        .save_prepared(
            op.payout_id,
            worker,
            &PreparedTransaction {
                payout_id: op.payout_id,
                txid: format!("{:064x}", op.payout_id.as_u128()),
                raw_transaction: vec![1, 2, 3],
            },
        )
        .await
        .unwrap();
    store.mark_broadcast(op.payout_id, worker).await.unwrap();
    store
        .reconcile_job(
            op.payout_id,
            ChainTxStatus::Mined {
                height: 99,
                confirmations: 3,
            },
            3,
        )
        .await
        .unwrap();
    store
        .update_balance(BalanceSnapshot {
            room_id: room,
            confirmed_zat: 1_890_000,
            spendable_zat: 1_890_000,
            chain_height: 101,
            scanned_height: 101,
        })
        .await
        .unwrap();
    assert_eq!(store.summary(room, owner).await.unwrap().reserved_zat, 0);
    store
        .reconcile_job(op.payout_id, ChainTxStatus::Reorged, 3)
        .await
        .unwrap();
    let summary = store.summary(room, owner).await.unwrap();
    assert_eq!(summary.reserved_zat, 1_100_000);
    assert_eq!(summary.funded_confirmed_count, 0);
    assert!(
        store
            .room(room, owner)
            .await
            .unwrap()
            .balance_updated_at
            .is_none()
    );
    let ciphertext: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT encrypted_ephemeral_receiver FROM payout_jobs WHERE id=$1")
            .bind(op.payout_id)
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(ciphertext.is_none());
}
