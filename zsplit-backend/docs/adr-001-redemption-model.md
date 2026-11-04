# ADR 001: reserve first, fund on redemption

Accepted: 2026-11-04. Both repository specifications were read completely before implementation. The architecture specification takes precedence over older prefunded-key notes in the product brief.

Tickets reserve confirmed spendable room funds plus a conservative per-payout fee budget. Ticket redemption chooses an ephemeral Orchard unified receiving address and atomically inserts one immutable payout intent in PostgreSQL. Keys and personal sweep destinations never enter the server. PostgreSQL stores application liabilities, while official Zcash SQLite APIs own notes, witnesses and wallet transactions.

One combined API process owns the native wallet, sync and serialized payout dispatcher. An exclusive wallet-directory process lock prevents accidental duplicate coordinators. Proofs run behind bounded blocking work. A dedicated worker library keeps those operations away from request handlers.

Exactly-once network effects cannot be created by SQL transactions alone. A durable native intent marker precedes SDK transaction construction. Signed transaction bytes are persisted before broadcast; a timeout permits only rebroadcast of identical bytes. An interrupted construction that cannot be tied conclusively to a stored transaction requires operator review, preserving the reservation. Availability is sacrificed to prevent duplicate funding. No timeout, expiration or chain reorganization permits blind replacement of a payment.

Funding confirmation comes only from wallet synchronization and chain transaction inclusion at the required depth. A client sweep report is a separate unverified flag. Reorganizations can downgrade funding status. Unredeemed expiry releases reservations and does not revoke a Zcash spending key or claw back funded notes.

The development frontend already present in the repository uses mock payments. It must remain visibly a demo until a real browser wallet receives and sweeps on the selected network. No mainnet readiness or audit is claimed.
