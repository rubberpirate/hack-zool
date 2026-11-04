//! Serialized, durable payout dispatch. A failed prepare is never blindly repeated.
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use uuid::Uuid;
use zsplit_claims::ReceiverCipher;
use zsplit_domain::{DomainError, FundingCommand, PayoutEngine, PreparedTransaction, Zats};
use zsplit_store::{PayoutJob, Store};

#[derive(Clone, Copy)]
pub struct WorkerConfig {
    pub confirmations: u32,
    pub poll_interval: Duration,
}
impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            confirmations: 3,
            poll_interval: Duration::from_secs(10),
        }
    }
}

pub async fn run(
    store: Store,
    engine: Arc<dyn PayoutEngine>,
    cipher: Arc<ReceiverCipher>,
    config: WorkerConfig,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), DomainError> {
    if config.confirmations == 0 || config.poll_interval.is_zero() {
        return Err(DomainError::InvalidInput);
    }
    let worker = Uuid::new_v4();
    store.recover_stale_jobs().await?;
    let mut ticker = tokio::time::interval(config.poll_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { return Ok(()); }
            }
            _ = ticker.tick() => {
                if *shutdown.borrow() { return Ok(()); }
                // Stop payouts when sync fails: stale balance must never authorize new spends.
                match engine.sync().await {
                    Ok(balances) => {
                        for balance in balances { store.update_balance(balance).await?; }
                    }
                    Err(_) => {
                        tracing::warn!(code = "wallet_sync_unavailable");
                        continue;
                    }
                }
                store.recover_stale_jobs().await?;
                reconcile(&store, engine.as_ref(), config.confirmations).await?;
                if let Some(job) = store.lease_job(worker).await? {
                    dispatch(&store, engine.as_ref(), cipher.as_ref(), worker, job).await?;
                }
            }
        }
    }
}

pub async fn dispatch(
    store: &Store,
    engine: &dyn PayoutEngine,
    cipher: &ReceiverCipher,
    worker: Uuid,
    job: PayoutJob,
) -> Result<(), DomainError> {
    let prepared = if let (Some(txid), Some(raw_transaction)) = (&job.txid, &job.raw_transaction) {
        PreparedTransaction {
            payout_id: job.id,
            txid: txid.clone(),
            raw_transaction: raw_transaction.clone(),
        }
    } else {
        // The wallet intent journal supplies recovery when transaction creation committed to SQLite.
        match engine.recover_prepared(job.id).await {
            Ok(Some(tx)) => tx,
            Err(_) => {
                store.mark_manual_review(job.id, worker).await?;
                return Ok(());
            }
            Ok(None) => {
                let receiver = match cipher.decrypt(job.claim_id, &job.encrypted_receiver) {
                    Ok(value) => value,
                    Err(_) => {
                        store.mark_manual_review(job.id, worker).await?;
                        return Ok(());
                    }
                };
                let command = FundingCommand {
                    payout_id: job.id,
                    room_id: job.room_id,
                    network: job.network,
                    amount: Zats::new(
                        u64::try_from(job.amount_zat).map_err(|_| DomainError::InvalidInput)?,
                    )?,
                    ephemeral_receiver: receiver,
                };
                match engine.prepare_payout(command).await {
                    Ok(tx) => tx,
                    Err(_) => {
                        // A prepare error can follow a durable native commit; operator reconciliation is required.
                        store.mark_manual_review(job.id, worker).await?;
                        tracing::warn!(payout_id = %job.id, code = "prepare_requires_review");
                        return Ok(());
                    }
                }
            }
        }
    };
    if prepared.payout_id != job.id
        || prepared.txid.len() != 64
        || prepared.raw_transaction.is_empty()
    {
        store.mark_manual_review(job.id, worker).await?;
        return Ok(());
    }
    // Commit before external effects; failure leaves native intent recoverable and prevents broadcast.
    store.save_prepared(job.id, worker, &prepared).await?;
    match engine.broadcast(&prepared).await {
        Ok(()) => store.mark_broadcast(job.id, worker).await?,
        Err(_) => {
            // Uncertain network result retains immutable signed bytes. Retry only the identical tx.
            store.defer_job(job.id, worker).await?;
            tracing::warn!(payout_id = %job.id, code = "broadcast_outcome_unknown");
        }
    }
    Ok(())
}

async fn reconcile(
    store: &Store,
    engine: &dyn PayoutEngine,
    confirmations: u32,
) -> Result<(), DomainError> {
    for job in store.tracked_jobs().await? {
        if let Some(txid) = job.txid {
            match engine.transaction_status(&txid).await {
                Ok(status) => store.reconcile_job(job.id, status, confirmations).await?,
                Err(_) => {
                    tracing::warn!(payout_id = %job.id, code = "transaction_status_unavailable")
                }
            }
        }
    }
    Ok(())
}
