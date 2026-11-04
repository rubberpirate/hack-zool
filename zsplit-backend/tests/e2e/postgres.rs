//! Real PostgreSQL safety tests. All synthetic chain values live only in this test harness.
use async_trait::async_trait;
use sqlx::Row;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;
use zsplit_domain::{
    BalanceSnapshot, ChainTxStatus, DepositAddress, DomainError, FundingCommand, PayoutEngine,
    PreparedTransaction,
};
use zsplit_integration::Fixture;

#[derive(Default)]
struct TestEngine {
    prepared: AtomicUsize,
    broadcasts: Mutex<Vec<Vec<u8>>>,
    fail_broadcast: bool,
    fail_prepare: bool,
    insufficient_funds: bool,
    journal_exists: bool,
}
#[async_trait]
impl PayoutEngine for TestEngine {
    async fn deposit_address(&self, _: Uuid) -> Result<DepositAddress, DomainError> {
        Err(DomainError::Unavailable)
    }
    async fn sync(&self) -> Result<Vec<BalanceSnapshot>, DomainError> {
        Err(DomainError::Unavailable)
    }
    async fn balance(&self, _: Uuid) -> Result<BalanceSnapshot, DomainError> {
        Err(DomainError::Unavailable)
    }
    async fn prepare_payout(
        &self,
        command: FundingCommand,
    ) -> Result<PreparedTransaction, DomainError> {
        self.prepared.fetch_add(1, Ordering::SeqCst);
        if self.fail_prepare {
            return Err(DomainError::ReconciliationRequired);
        }
        if self.insufficient_funds {
            return Err(DomainError::InsufficientFunds);
        }
        Ok(test_transaction(command.payout_id))
    }
    async fn broadcast(&self, tx: &PreparedTransaction) -> Result<(), DomainError> {
        self.broadcasts
            .lock()
            .unwrap()
            .push(tx.raw_transaction.clone());
        if self.fail_broadcast {
            Err(DomainError::Unavailable)
        } else {
            Ok(())
        }
    }
    async fn transaction_status(&self, _: &str) -> Result<ChainTxStatus, DomainError> {
        Ok(ChainTxStatus::Unknown)
    }
    async fn recover_prepared(&self, _: Uuid) -> Result<Option<PreparedTransaction>, DomainError> {
        if self.journal_exists {
            return Err(DomainError::ReconciliationRequired);
        }
        Ok(None)
    }
}
fn test_transaction(id: Uuid) -> PreparedTransaction {
    // Invalid chain bytes on purpose: test fixtures cannot be mistaken for genuine Zcash transactions.
    PreparedTransaction {
        payout_id: id,
        txid: format!("{}{}", id.simple(), id.simple()),
        raw_transaction: b"TEST-ONLY-NOT-A-ZCASH-TX".to_vec(),
    }
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn hundred_competing_redemptions_produce_one_intent() {
    let f = Fixture::new(50).await;
    f.issue().await;
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let store = f.store.clone();
        let command = f.redeem(0, Uuid::new_v4());
        tasks.push(tokio::spawn(async move {
            store.enqueue_redemption(command).await
        }));
    }
    let mut wins = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => wins += 1,
            Err(DomainError::Conflict) => {}
            Err(other) => panic!("unexpected redemption result: {other}"),
        }
    }
    assert_eq!(wins, 1);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM payout_jobs WHERE claim_id=$1")
        .bind(f.claims[0].id)
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        10_000_000
    );
    assert!(matches!(
        f.store.summary(f.room, Uuid::new_v4()).await,
        Err(DomainError::NotFound)
    ));
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn identical_retries_and_receiver_mismatch_are_safe() {
    let f = Fixture::new(1).await;
    f.issue().await;
    let key = Uuid::new_v4();
    let first = f.store.enqueue_redemption(f.redeem(0, key)).await.unwrap();
    for _ in 0..20 {
        let retry = f.store.enqueue_redemption(f.redeem(0, key)).await.unwrap();
        assert_eq!(retry.payout_id, first.payout_id);
    }
    let mut mismatch = f.redeem(0, key);
    mismatch.request_fingerprint = vec![99; 32];
    assert!(matches!(
        f.store.enqueue_redemption(mismatch).await,
        Err(DomainError::Conflict)
    ));
    assert!(matches!(
        f.store
            .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
            .await,
        Err(DomainError::Conflict)
    ));
    assert!(
        f.store
            .report_sweep(f.claims[0].id, &f.claims[0].token_digest)
            .await
            .is_err()
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn issuance_is_atomic_and_cannot_reserve_twice() {
    let f = Fixture::new(50).await;
    let mut broken = f.claims.clone();
    broken[49].token_digest = broken[0].token_digest;
    assert!(
        f.store
            .issue_claims(f.room, f.owner, &broken)
            .await
            .is_err()
    );
    let summary = f.store.summary(f.room, f.owner).await.unwrap();
    assert_eq!(summary.reserved_zat, 0);
    assert_eq!(summary.reserved_count, 0);
    let (a, b) = tokio::join!(
        f.store.issue_claims(f.room, f.owner, &f.claims),
        f.store.issue_claims(f.room, f.owner, &f.claims)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let summary = f.store.summary(f.room, f.owner).await.unwrap();
    assert_eq!(summary.reserved_zat, 10_000_000);
    assert_eq!(summary.reserved_count, 50);
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn competing_workers_never_lease_same_job() {
    let f = Fixture::new(50).await;
    f.issue().await;
    for index in 0..50 {
        f.store
            .enqueue_redemption(f.redeem(index, Uuid::new_v4()))
            .await
            .unwrap();
    }
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let store = f.store.clone();
        tasks.push(tokio::spawn(async move {
            store.lease_job(Uuid::new_v4()).await.unwrap().map(|j| j.id)
        }));
    }
    let mut seen = std::collections::HashSet::new();
    for task in tasks {
        if let Some(id) = task.await.unwrap() {
            assert!(seen.insert(id));
        }
    }
    assert_eq!(seen.len(), 50);
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn interrupted_construction_is_quarantined_but_prepared_bytes_recover() {
    let f = Fixture::new(2).await;
    f.issue().await;
    for index in 0..2 {
        f.store
            .enqueue_redemption(f.redeem(index, Uuid::new_v4()))
            .await
            .unwrap();
    }
    let worker = Uuid::new_v4();
    let first = f.store.lease_job(worker).await.unwrap().unwrap();
    let second = f.store.lease_job(worker).await.unwrap().unwrap();
    let prepared = test_transaction(second.id);
    f.store
        .save_prepared(second.id, worker, &prepared)
        .await
        .unwrap();
    sqlx::query("UPDATE payout_jobs SET lease_until=now()-interval '1 second'")
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(f.store.recover_stale_jobs().await.unwrap(), 2);
    let state: String = sqlx::query_scalar("SELECT state FROM payout_jobs WHERE id=$1")
        .bind(first.id)
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(state, "manual_review");
    let restarted = Uuid::new_v4();
    let recovered = f.store.lease_job(restarted).await.unwrap().unwrap();
    assert_eq!(recovered.id, second.id);
    let engine = TestEngine::default();
    zsplit_worker::dispatch(&f.store, &engine, &f.cipher, restarted, recovered)
        .await
        .unwrap();
    assert_eq!(engine.prepared.load(Ordering::SeqCst), 0);
    assert_eq!(
        *engine.broadcasts.lock().unwrap(),
        vec![prepared.raw_transaction]
    );
    assert!(f.store.lease_job(Uuid::new_v4()).await.unwrap().is_none());
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn ambiguous_broadcast_retries_identical_bytes_without_second_prepare() {
    let f = Fixture::new(1).await;
    f.issue().await;
    f.store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    let engine = Arc::new(TestEngine {
        fail_broadcast: true,
        ..Default::default()
    });
    for _ in 0..3 {
        let worker = Uuid::new_v4();
        let job = f.store.lease_job(worker).await.unwrap().unwrap();
        zsplit_worker::dispatch(&f.store, engine.as_ref(), &f.cipher, worker, job)
            .await
            .unwrap();
        sqlx::query("UPDATE payout_jobs SET next_attempt_at=now()-interval '1 second'")
            .execute(f.store.pool())
            .await
            .unwrap();
    }
    assert_eq!(engine.prepared.load(Ordering::SeqCst), 1);
    let broadcasts = engine.broadcasts.lock().unwrap().clone();
    assert_eq!(broadcasts.len(), 3);
    assert!(broadcasts.windows(2).all(|pair| pair[0] == pair[1]));
    let row = sqlx::query("SELECT encrypted_ephemeral_receiver,state FROM payout_jobs")
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert!(
        row.get::<Option<Vec<u8>>, _>("encrypted_ephemeral_receiver")
            .is_none()
    );
    assert_eq!(row.get::<String, _>("state"), "prepared");
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn proof_error_never_requeues_uncertain_construction() {
    let f = Fixture::new(1).await;
    f.issue().await;
    f.store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    let engine = TestEngine {
        fail_prepare: true,
        ..Default::default()
    };
    let worker = Uuid::new_v4();
    let job = f.store.lease_job(worker).await.unwrap().unwrap();
    zsplit_worker::dispatch(&f.store, &engine, &f.cipher, worker, job)
        .await
        .unwrap();
    assert_eq!(engine.prepared.load(Ordering::SeqCst), 1);
    assert!(engine.broadcasts.lock().unwrap().is_empty());
    assert!(f.store.lease_job(Uuid::new_v4()).await.unwrap().is_none());
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        200_000
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn confirmation_reorg_and_sweep_report_preserve_liabilities() {
    let f = Fixture::new(1).await;
    f.issue().await;
    let redemption = f
        .store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    let worker = Uuid::new_v4();
    let job = f.store.lease_job(worker).await.unwrap().unwrap();
    let engine = TestEngine::default();
    zsplit_worker::dispatch(&f.store, &engine, &f.cipher, worker, job)
        .await
        .unwrap();
    f.store
        .reconcile_job(
            redemption.payout_id,
            ChainTxStatus::Mined {
                height: 101,
                confirmations: 3,
            },
            3,
        )
        .await
        .unwrap();
    let balance = BalanceSnapshot {
        room_id: f.room,
        confirmed_zat: 99_800_000,
        spendable_zat: 99_800_000,
        chain_height: 103,
        scanned_height: 103,
    };
    f.store.update_balance(balance.clone()).await.unwrap();
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        0
    );
    f.store
        .report_sweep(f.claims[0].id, &f.claims[0].token_digest)
        .await
        .unwrap();
    f.store
        .reconcile_job(redemption.payout_id, ChainTxStatus::Reorged, 3)
        .await
        .unwrap();
    let summary = f.store.summary(f.room, f.owner).await.unwrap();
    assert_eq!(summary.funded_confirmed_count, 0);
    assert_eq!(summary.sweep_reported_count, 1);
    assert_eq!(summary.reserved_zat, 200_000);
    assert!(
        f.store
            .enqueue_withdrawal(
                f.room,
                f.owner,
                1_000_000,
                Uuid::new_v4(),
                vec![1],
                vec![1; 32]
            )
            .await
            .is_err()
    );
    f.store
        .reconcile_job(redemption.payout_id, ChainTxStatus::Reorged, 3)
        .await
        .unwrap();
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        200_000
    );
    f.store
        .reconcile_job(
            redemption.payout_id,
            ChainTxStatus::Mined {
                height: 102,
                confirmations: 3,
            },
            3,
        )
        .await
        .unwrap();
    f.store.update_balance(balance).await.unwrap();
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        0
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn expiration_releases_only_unredeemed_and_withdrawals_serialize() {
    let f = Fixture::new(2).await;
    f.issue().await;
    f.store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    sqlx::query("UPDATE claims SET expires_at=now()-interval '1 second'")
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(f.store.expire_claims().await.unwrap(), 1);
    assert_eq!(f.store.expire_claims().await.unwrap(), 0);
    let summary = f.store.summary(f.room, f.owner).await.unwrap();
    assert_eq!(summary.reserved_zat, 200_000);
    assert_eq!(summary.expired_count, 1);
    assert_eq!(summary.pending_count, 1);
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let store = f.store.clone();
        let room = f.room;
        let owner = f.owner;
        tasks.push(tokio::spawn(async move {
            store
                .enqueue_withdrawal(
                    room,
                    owner,
                    60_000_000,
                    Uuid::new_v4(),
                    vec![1],
                    vec![1; 32],
                )
                .await
        }));
    }
    let mut wins = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            wins += 1;
        }
    }
    assert_eq!(wins, 1);
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        60_300_000
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn definitely_unprepared_fund_wait_preserves_intent_and_can_retry() {
    let f = Fixture::new(1).await;
    f.issue().await;
    let intent = f
        .store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    let worker = Uuid::new_v4();
    let job = f.store.lease_job(worker).await.unwrap().unwrap();
    let engine = TestEngine {
        insufficient_funds: true,
        ..Default::default()
    };
    zsplit_worker::dispatch(&f.store, &engine, &f.cipher, worker, job)
        .await
        .unwrap();
    let row=sqlx::query("SELECT state,txid,encrypted_ephemeral_receiver IS NOT NULL receiver_kept FROM payout_jobs WHERE id=$1").bind(intent.payout_id).fetch_one(f.store.pool()).await.unwrap();
    assert_eq!(row.get::<String, _>("state"), "queued");
    assert!(row.get::<Option<String>, _>("txid").is_none());
    assert!(row.get::<bool, _>("receiver_kept"));
    assert!(engine.broadcasts.lock().unwrap().is_empty());
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        200_000
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn manual_recovery_refuses_native_intents_and_retains_reservations() {
    let f = Fixture::new(1).await;
    f.issue().await;
    let intent = f
        .store
        .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
        .await
        .unwrap();
    let worker = Uuid::new_v4();
    f.store.lease_job(worker).await.unwrap().unwrap();
    f.store
        .mark_manual_review(intent.payout_id, worker)
        .await
        .unwrap();
    let ambiguous = TestEngine {
        journal_exists: true,
        ..Default::default()
    };
    assert!(
        zsplit_worker::requeue_verified_unprepared(&f.store, &ambiguous, intent.payout_id)
            .await
            .is_err()
    );
    let state: String = sqlx::query_scalar("SELECT state FROM payout_jobs WHERE id=$1")
        .bind(intent.payout_id)
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(state, "manual_review");
    zsplit_worker::requeue_verified_unprepared(&f.store, &TestEngine::default(), intent.payout_id)
        .await
        .unwrap();
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        200_000
    );
    let job = f.store.lease_job(worker).await.unwrap().unwrap();
    assert_eq!(job.id, intent.payout_id);
    f.store
        .save_prepared(job.id, worker, &test_transaction(job.id))
        .await
        .unwrap();
    f.store.mark_manual_review(job.id, worker).await.unwrap();
    assert!(
        zsplit_worker::requeue_verified_unprepared(&f.store, &TestEngine::default(), job.id)
            .await
            .is_err()
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and real PostgreSQL"]
async fn spent_balances_do_not_remain_at_a_historical_peak() {
    let f = Fixture::new(1).await;
    f.issue().await;
    f.store
        .update_balance(BalanceSnapshot {
            room_id: f.room,
            confirmed_zat: 50_000_000,
            spendable_zat: 40_000_000,
            chain_height: 101,
            scanned_height: 101,
        })
        .await
        .unwrap();
    let current = f.store.summary(f.room, f.owner).await.unwrap();
    assert_eq!(current.confirmed_zat, 50_000_000);
    assert_eq!(current.spendable_zat, 40_000_000);
    f.store
        .update_balance(BalanceSnapshot {
            room_id: f.room,
            confirmed_zat: 0,
            spendable_zat: 0,
            chain_height: 102,
            scanned_height: 102,
        })
        .await
        .unwrap();
    assert_eq!(f.store.room(f.room, f.owner).await.unwrap().state, "paused");
    assert!(
        f.store
            .enqueue_redemption(f.redeem(0, Uuid::new_v4()))
            .await
            .is_err()
    );
    assert_eq!(
        f.store.summary(f.room, f.owner).await.unwrap().reserved_zat,
        200_000
    );
    f.finish().await;
}
