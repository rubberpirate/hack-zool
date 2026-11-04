use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

pub type RoomId = Uuid;
pub type ClaimId = Uuid;
pub type PayoutId = Uuid;
pub type OrganizerId = Uuid;
pub const MAX_ZATS: u64 = 2_100_000_000_000_000;
pub const MAX_TICKETS: usize = 50;
/// Default marginal transaction fee reserve per ticket (10,000 Zatoshis = 0.0001 ZEC, ZIP-317 standard minimum).
pub const DEFAULT_FEE_RESERVE: u64 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}
impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Regtest => "regtest",
        })
    }
}
impl std::str::FromStr for Network {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            "regtest" => Ok(Self::Regtest),
            _ => Err(DomainError::InvalidInput),
        }
    }
}
impl TryFrom<String> for Network {
    type Error = DomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Zats(u64);
impl<'de> Deserialize<'de> for Zats {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}
impl Zats {
    pub fn new(v: u64) -> Result<Self, DomainError> {
        if v <= MAX_ZATS {
            Ok(Self(v))
        } else {
            Err(DomainError::InvalidInput)
        }
    }
    pub fn value(self) -> u64 {
        self.0
    }
    pub fn checked_add(self, rhs: Self) -> Result<Self, DomainError> {
        Self::new(self.0.checked_add(rhs.0).ok_or(DomainError::InvalidInput)?)
    }
    pub fn checked_sub(self, rhs: Self) -> Result<Self, DomainError> {
        Self::new(
            self.0
                .checked_sub(rhs.0)
                .ok_or(DomainError::InsufficientFunds)?,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Reserved,
    FundingQueued,
    Preparing,
    FundingBroadcast,
    FundedUnconfirmed,
    FundedConfirmed,
    Expired,
    Cancelled,
    AmbiguousBroadcast,
    Reconciling,
    Reorged,
    ManualReview,
}
impl ClaimStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::FundingQueued => "funding_queued",
            Self::Preparing => "preparing",
            Self::FundingBroadcast => "funding_broadcast",
            Self::FundedUnconfirmed => "funded_unconfirmed",
            Self::FundedConfirmed => "funded_confirmed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::AmbiguousBroadcast => "ambiguous_broadcast",
            Self::Reconciling => "reconciling",
            Self::Reorged => "reorged",
            Self::ManualReview => "manual_review",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    AwaitingFunds,
    Ready,
    Paused,
    Closed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub room_id: RoomId,
    pub confirmed_zat: u64,
    pub spendable_zat: u64,
    pub chain_height: u32,
    pub scanned_height: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepositAddress {
    pub room_id: RoomId,
    pub address: String,
    pub network: Network,
    pub birthday_height: u32,
}
pub struct FundingCommand {
    pub payout_id: PayoutId,
    pub room_id: RoomId,
    pub network: Network,
    pub amount: Zats,
    pub ephemeral_receiver: Zeroizing<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PreparedTransaction {
    pub payout_id: PayoutId,
    pub txid: String,
    pub raw_transaction: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChainTxStatus {
    Unknown,
    Mempool,
    Mined { height: u32, confirmations: u32 },
    Expired,
    Reorged,
}
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid input")]
    InvalidInput,
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("operation conflicts with current state")]
    Conflict,
    #[error("insufficient confirmed funds")]
    InsufficientFunds,
    #[error("claim expired")]
    Expired,
    #[error("network mismatch")]
    NetworkMismatch,
    #[error("dependency unavailable")]
    Unavailable,
    #[error("manual reconciliation required")]
    ReconciliationRequired,
    #[error("storage failure")]
    Storage,
}
#[async_trait]
pub trait PayoutEngine: Send + Sync {
    async fn deposit_address(&self, room: RoomId) -> Result<DepositAddress, DomainError>;
    async fn sync(&self) -> Result<Vec<BalanceSnapshot>, DomainError>;
    async fn balance(&self, room: RoomId) -> Result<BalanceSnapshot, DomainError>;
    async fn prepare_payout(
        &self,
        command: FundingCommand,
    ) -> Result<PreparedTransaction, DomainError>;
    async fn broadcast(&self, tx: &PreparedTransaction) -> Result<(), DomainError>;
    async fn transaction_status(&self, txid: &str) -> Result<ChainTxStatus, DomainError>;
    async fn recover_prepared(
        &self,
        payout: PayoutId,
    ) -> Result<Option<PreparedTransaction>, DomainError>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claim {
    pub id: ClaimId,
    pub room_id: RoomId,
    pub amount_zat: u64,
    pub state: ClaimStatus,
    pub expires_at: DateTime<Utc>,
    pub payout_id: Option<PayoutId>,
    pub sweep_reported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_is_checked() {
        assert!(serde_json::from_str::<Zats>("2100000000000001").is_err());
        assert!(serde_json::from_str::<Zats>("-1").is_err());
        assert!(serde_json::from_str::<Zats>("0.1").is_err());
        assert!(Zats::new(MAX_ZATS + 1).is_err());
        assert!(
            Zats::new(MAX_ZATS)
                .unwrap()
                .checked_add(Zats::new(1).unwrap())
                .is_err()
        );
        assert!(
            Zats::new(0)
                .unwrap()
                .checked_sub(Zats::new(1).unwrap())
                .is_err()
        );
    }
}
