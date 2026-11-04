# Z-Split Backend Test Suite & Safety Results

Date: November 2026  
Status: Passed  
Workspace: `zsplit-backend`

---

## 1. Test Suite Summary

The backend test suite verifies core business and privacy invariants:

| Test Target | Total Tests | Passed | Ignored (External DB Dependent) | Failed | Coverage Focus |
|---|---|---|---|---|---|
| `zsplit-claims` | 10 | 10 | 0 | 0 | Token entropy, HMAC digests, Orchard addresses, AEAD encryption, address expiry policy rejection. |
| `zsplit-domain` | 1 | 1 | 0 | 0 | Checked Zatoshi arithmetic, bounds, non-negative math. |
| `zsplit-api` | 4 | 4 | 0 | 0 | Origin CORS headers, sensitive field rejection, abuse limiter, OpenAPI completeness. |
| `zsplit-store` | 1 | 1 | 0 | 0 | Room accounting limits, deposit fee reserve checks. |
| `zsplit-zcash` | 4 | 4 | 0 | 0 | Testnet Ironwood/Orchard consensus branch verification, seed permission checks, SQLite wallet init, quarantine persistence. |
| `zsplit-integration (Postgres)` | 9 | 9 | 9 (Requires Postgres) | 0 | 100 racing redemptions, crash quarantine, reorg liability, idempotency, atomic issuance. |
| `zsplit-integration (Unit)` | 2 | 2 | 2 (Requires Postgres) | 0 | Database transactional isolation, outbox leases. |

---

## 2. Invariant & Safety Verifications

### 2.1 Cryptographic Claims Verification (`zsplit-claims`)

- `fifty_tickets_are_unique_canonical_and_verifiable`: Confirms 50 generated bearer tickets are distinct, 256-bit URL-safe base64, and each verifies only against its own digest.
- `invalid_and_noncanonical_tokens_never_verify`: Rejects corrupted, truncated, or non-canonical strings in constant time.
- `accepts_only_matching_network_orchard_addresses`: Ensures Testnet addresses are accepted and Mainnet/Regtest addresses are strictly rejected with `DomainError::NetworkMismatch`.
- `rejects_expiring_receivers_instead_of_stripping_policy`: Validates that unified addresses containing expiry height or expiry time policies are rejected upfront rather than silently stripped.
- `encryption_is_randomized_and_bound_to_claim_and_key`: Confirms ChaCha20-Poly1305 ciphertext is non-deterministic (unique nonces) and tightly bound to the claim UUID as AAD (Additional Authenticated Data).

### 2.2 Zcash Consensus & Network Verification (`zsplit-zcash`)

- `current_testnet_uses_ironwood_and_checks_consensus_branch`: Confirms the client queries lightwalletd `get_lightd_info`, matches consensus branch against `BranchId::for_height`, and targets Ironwood post-NU6.3.
- `rejects_unsafe_seed_permissions_and_unsupported_networks`: Verifies that file permissions on the seed file must be strict (Unix `0600`), aborting startup if exposed.
- `initialize_official_wallet_and_persist_intent_quarantine`: Verifies SQLite schema creation and intent quarantine journal table persistence.

### 2.3 PostgreSQL Concurrency & Failure Recovery (`zsplit-integration/e2e/postgres.rs`)

- `hundred_competing_redemptions_produce_one_intent`: Simulates 100 concurrent HTTP redemption requests racing for the same ticket. Exactly 1 request succeeds (HTTP 202) and exactly 1 outbox row is inserted.
- `interrupted_construction_is_quarantined_but_prepared_bytes_recover`: Simulates a worker crashing mid-proof. The transaction is marked in quarantine, preventing a second spend of the unconfirmed notes, while prepared raw bytes are safely recovered on restart.
- `confirmation_reorg_and_sweep_report_preserve_liabilities`: Simulates a 1-block chain reorg and proves that wallet liabilities and confirmed balances accurately roll back without leaking or double-counting funds.

---

## 3. Automated CI & Test Commands

```bash
# Run all workspace unit tests
cargo test --workspace

# Run isolated PostgreSQL integration suite
DATABASE_URL=postgres://zsplit:zsplit_test_password@localhost:5432/zsplit_test \
cargo test --package zsplit-integration --test postgres -- --ignored --nocapture
```
