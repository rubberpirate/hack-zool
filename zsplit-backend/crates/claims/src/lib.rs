//! Bearer-ticket policy and encrypted ephemeral receivers.
//!
//! Database row locks and unique payout constraints enforce one-time redemption;
//! these helpers deliberately do not claim to provide persistence or authorization.

mod lifecycle;
mod receiver;
mod token;

pub use lifecycle::{can_transition, is_expirable, validate_redemption};
pub use receiver::{ReceiverCipher, validate_receiver};
pub use token::{IssuedToken, TOKEN_BYTES, TOKEN_ENCODED_LEN, TokenService};
