use chrono::{DateTime, Utc};
use zsplit_domain::{ClaimStatus, DomainError, Network};

/// Allowed financial state transitions. Sweep reports are a separate flag.
/// `false` for self transitions: idempotency returns the existing operation instead.
pub fn can_transition(from: ClaimStatus, to: ClaimStatus) -> bool {
    use ClaimStatus::*;
    matches!(
        (from, to),
        (Reserved, FundingQueued | Expired | Cancelled)
            | (FundingQueued, Preparing | Cancelled)
            | (
                Preparing,
                FundingBroadcast | AmbiguousBroadcast | ManualReview
            )
            | (
                FundingBroadcast,
                FundedUnconfirmed | FundedConfirmed | AmbiguousBroadcast | Reorged | Reconciling
            )
            | (FundedUnconfirmed, FundedConfirmed | Reorged | Reconciling)
            | (FundedConfirmed, Reorged | Reconciling)
            | (
                AmbiguousBroadcast,
                FundingBroadcast | FundedUnconfirmed | FundedConfirmed | Reconciling | ManualReview
            )
            | (Reorged, Reconciling | FundedUnconfirmed | FundedConfirmed)
            | (
                Reconciling,
                FundingBroadcast | FundedUnconfirmed | FundedConfirmed | Reorged | ManualReview
            )
            | (ManualReview, Reconciling)
    )
}

/// Expiry only releases a reservation that never entered the payout queue.
pub fn is_expirable(state: ClaimStatus, expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    state == ClaimStatus::Reserved && expires_at <= now
}

/// A first redemption requires a persisted client wallet and an unexpired reservation.
/// Callers must validate the token and perform this check under the claim row lock.
/// Idempotent retries are handled by the repository before invoking this policy.
pub fn validate_redemption(
    state: ClaimStatus,
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
    room_network: Network,
    client_network: Network,
    wallet_persisted: bool,
) -> Result<(), DomainError> {
    if room_network != client_network {
        return Err(DomainError::NetworkMismatch);
    }
    if !wallet_persisted {
        return Err(DomainError::InvalidInput);
    }
    if state != ClaimStatus::Reserved {
        return Err(DomainError::Conflict);
    }
    if expires_at <= now {
        return Err(DomainError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    const STATES: [ClaimStatus; 12] = [
        ClaimStatus::Reserved,
        ClaimStatus::FundingQueued,
        ClaimStatus::Preparing,
        ClaimStatus::FundingBroadcast,
        ClaimStatus::FundedUnconfirmed,
        ClaimStatus::FundedConfirmed,
        ClaimStatus::Expired,
        ClaimStatus::Cancelled,
        ClaimStatus::AmbiguousBroadcast,
        ClaimStatus::Reconciling,
        ClaimStatus::Reorged,
        ClaimStatus::ManualReview,
    ];

    #[test]
    fn expiry_never_releases_an_inflight_or_funded_payout() {
        let now = Utc::now();
        for state in STATES {
            assert_eq!(
                is_expirable(state, now, now),
                state == ClaimStatus::Reserved
            );
            assert!(!is_expirable(state, now + Duration::seconds(1), now));
        }
    }

    #[test]
    fn no_state_can_restart_a_claim_or_expire_a_live_payout() {
        for from in STATES {
            assert!(!can_transition(from, ClaimStatus::Reserved));
            assert!(!can_transition(from, from));
            if from != ClaimStatus::Reserved {
                assert!(!can_transition(from, ClaimStatus::Expired));
                assert!(!can_transition(from, ClaimStatus::FundingQueued));
            }
            assert!(!can_transition(ClaimStatus::Expired, from));
            assert!(!can_transition(ClaimStatus::Cancelled, from));
        }
        assert!(can_transition(
            ClaimStatus::FundedConfirmed,
            ClaimStatus::Reorged
        ));
        assert!(can_transition(
            ClaimStatus::Preparing,
            ClaimStatus::ManualReview
        ));
    }

    #[test]
    fn redemption_requires_persistence_network_and_a_live_reservation() {
        let now = Utc::now();
        let later = now + Duration::minutes(1);
        assert!(
            validate_redemption(
                ClaimStatus::Reserved,
                later,
                now,
                Network::Testnet,
                Network::Testnet,
                true
            )
            .is_ok()
        );
        assert!(matches!(
            validate_redemption(
                ClaimStatus::Reserved,
                now,
                now,
                Network::Testnet,
                Network::Testnet,
                true
            ),
            Err(DomainError::Expired)
        ));
        assert!(matches!(
            validate_redemption(
                ClaimStatus::Reserved,
                later,
                now,
                Network::Testnet,
                Network::Mainnet,
                true
            ),
            Err(DomainError::NetworkMismatch)
        ));
        assert!(matches!(
            validate_redemption(
                ClaimStatus::Reserved,
                later,
                now,
                Network::Testnet,
                Network::Testnet,
                false
            ),
            Err(DomainError::InvalidInput)
        ));
        assert!(matches!(
            validate_redemption(
                ClaimStatus::FundingQueued,
                later,
                now,
                Network::Testnet,
                Network::Testnet,
                true
            ),
            Err(DomainError::Conflict)
        ));
    }
}
