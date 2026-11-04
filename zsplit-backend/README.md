# Z-Split — Privacy-Preserving Shielded Group Payout Backend

> Pay 50 people in a room with shielded Zcash (Orchard / Ironwood) without ever collecting a single recipient wallet address.

[![CI](https://github.com/rubberpirate/hack-zool/actions/workflows/ci.yml/badge.svg)](https://github.com/rubberpirate/hack-zool/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)

---

## 1. Overview & Architecture

Z-Split solves the identity-to-address doxxing problem inherent in conventional group payouts (hackathon prizes, DAO distributions, bounty payouts). Instead of asking 50 winners for their addresses in a public form or spreadsheet, an organizer creates a room, deposits shielded ZEC into a coordinator wallet, and generates 50 one-time claim QR links.

### The Reserve-First, Fund-on-Redeem Architecture

```
[Organizer Client]
       │
       ├─ 1. POST /api/v1/rooms ──────────────► [Axum API] ──► [PostgreSQL: Draft Room]
       │                                            │
       ├─ 2. Funds Shielded Coordinator Address ───►┼──► [librustzcash Native Wallet]
       │                                            │           │
       │                                            ▼           ▼
       ├─ 3. POST /api/v1/rooms/{id}/claims ──► [HMAC Tokens] [Balance Sync >= Liabilities]
       │    (Bearer tickets returned ONCE)          │
       ▼                                            ▼
[Recipient Opens Claim Link: #t=TOKEN]      [Stored HMAC Digests]
       │
       ├─ 4. Client WASM derives fresh Ephemeral Keypair (Local ONLY)
       │
       ├─ 5. POST /api/v1/claims/{id}/redeem (Token + Ephemeral Receiver)
       │           │
       │           ▼
       │     [Constant-Time HMAC Match & Address Validation]
       │           │
       │           ▼
       │     [ChaCha20-Poly1305 AEAD Ephemeral Address Encryption]
       │           │
       │           ▼
       │     [Atomic PostgreSQL Reservation -> Outbox Intent]
       │           │
       │           ▼
       │     [Dedicated Bounded Wallet Worker]
       │           │
       │           ├─ Constructs & Proves Shielded Tx (LocalTxProver)
       │           ├─ Broadcasts via lightwalletd gRPC (https://testnet.zec.rocks:443)
       │           └─ Observes Confirmations (Policy: 3 blocks)
       │
       ▼
[Recipient Sweeps Ephemeral Wallet -> Personal Private Shielded Wallet]
       │
       └─ (Optional) POST /api/v1/claims/{id}/sweep-report
```

### Core Security & Privacy Invariants

1. **Zero Collection of Personal Addresses:** The backend never receives, persists, or transmits recipient spending keys, seeds, or personal destination addresses.
2. **Ephemeral Receiver Minimization:** The recipient's browser derives a fresh single-use shielded address for receiving funds. Pending addresses are encrypted at rest with ChaCha20-Poly1305 and deleted after outbox construction.
3. **Constant-Time Bearer Verification:** Claim tokens are 256-bit URL-safe base64 strings passed only via URL fragments (`#t=...`), hashed with server HMAC-SHA256, and verified using constant-time comparison (`subtle::ConstantTimeEq`).
4. **Idempotent Outbox & Double-Spend Quarantine:** Double-redemption races are eliminated via PostgreSQL row-level locks and unique constraints. If transaction construction is interrupted, the intent enters quarantine to prevent concurrent spend attempts.
5. **Non-Blocking Proving:** Zero-knowledge proof generation and key derivation run inside dedicated bounded worker threads, protecting Axum's Tokio async executor from latency spikes.

---

## 2. Workspace Module Map & Ownership

The repository is organized into a clean Cargo workspace:

| Crate / App | Module | Ownership / Purpose |
|---|---|---|
| `apps/api` | API & Documentation | Axum 0.8 HTTP API, Utoipa OpenAPI 3.1 docs, SSE streaming, authentication, rate limiting. |
| `crates/store` | Database & Ledger | PostgreSQL persistence with SQLx 0.9, transactional accounting, outbox lease queue, balance reconciliation. |
| `migrations` | Schema | Atomic PostgreSQL migrations for organizers, rooms, claims, outbox jobs, and balance snapshots. |
| `crates/zcash` | Zcash Core | Official `librustzcash` (0.25), `zcash_client_sqlite` (0.23), Orchard/Ironwood pool handling, lightwalletd Tonic gRPC, `LocalTxProver`. |
| `apps/worker` | Background Worker | Dedicated daemon leasing outbox intents, synchronizing chain state, and tracking block confirmations. |
| `crates/claims` | Claims & Privacy | Token generation (`TokenService`), receiver validation (`validate_receiver`), ChaCha20-Poly1305 cipher (`ReceiverCipher`). |
| `crates/domain` | Shared Domain Models | Strongly typed identifiers (`RoomId`, `ClaimId`), checked zatoshi math (`Zats`), error types (`DomainError`). |
| `tests` | Integration & E2E | Concurrency tests, crash recovery, reorg handling, and PostgreSQL test fixtures. |
| `benches` | Benchmarking | Reproducible microbenchmarks for token issuance, verification, encryption, and database operations. |

---

## 3. Quick Start & Setup

### Prerequisites

- **Rust 1.94+** (`rustc --version`, `cargo --version`)
- **Protobuf Compiler** (`protoc`)
- **SQLite 3** (`libsqlite3-dev`)
- **PostgreSQL 16+** (or Docker)

### 1. Configure Environment

Copy the environment template and adjust configuration if needed:

```bash
cp .env.example .env
```

### 2. Run with Docker Compose (Recommended)

Start PostgreSQL and the Z-Split combined API + Wallet Worker:

```bash
docker compose up -d
```

Check the service health:

```bash
curl http://localhost:8080/healthz
# {"status":"ok","network":"testnet"}
```

Access Swagger UI interactive documentation at:  
👉 **http://localhost:8080/docs**

### 3. Run Locally with Cargo

If running PostgreSQL locally:

```bash
export DATABASE_URL=postgres://zsplit:zsplit_dev_password@localhost:5432/zsplit
export ORGANIZER_ID=a0000000-0000-0000-0000-000000000001
export ORGANIZER_API_TOKEN=zsplit_local_dev_organizer_api_token_32chars
export CLAIM_TOKEN_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
export RECEIVER_ENCRYPTION_KEY=fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210
export ALLOWED_ORIGIN=http://localhost:5173
export PUBLIC_CLAIM_ORIGIN=http://localhost:5173
export ZCASH_SEED_FILE=./data/coordinator.seed
export ZCASH_DATA_DIR=./data/wallet
export ENABLE_DOCS=true

# Initialize coordinator seed file (first-time only)
cargo run --package zsplit-zcash --example wallet -- seed

# Start API & Worker
cargo run --package zsplit-api
```

---

## 4. Verification & Testing

### Code Quality Gates

```bash
# 1. Format Check
cargo fmt --all -- --check

# 2. Strict Clippy Lints
cargo clippy --workspace --all-targets -- -D warnings

# 3. Workspace Unit Tests
cargo test --workspace
```

### PostgreSQL Concurrency & Reorg Tests

With `DATABASE_URL` configured to a PostgreSQL instance:

```bash
cargo test --package zsplit-integration --test postgres -- --ignored --nocapture
```

Tests verify:
- Atomic issuance and double-reservation rejection.
- 100 racing redemptions resulting in exactly one payout intent.
- Receiver mismatch protection and crash quarantine.
- Unredeemed claim expiration and liability release.
- Chain reorganization accounting preservation.

---

## 5. Benchmarking & Performance

Run the release microbenchmark suite:

```bash
cargo run --package zsplit-integration --bin zsplit-bench --release
```

### Measured Performance (200 sample iterations)

| Operation | Metric | Latency |
|---|---|---|
| **50 Claim Tokens Generation** | p50 / p95 / p99 | **0.034 ms** / 0.035 ms / 0.041 ms |
| **50 Claim Tokens Verification** | p50 / p95 / p99 | **0.015 ms** / 0.016 ms / 0.019 ms |
| **50 Receivers ChaCha20-Poly1305 Encrypt** | p50 / p95 / p99 | **0.088 ms** / 0.092 ms / 0.094 ms |
| **50 Receivers ChaCha20-Poly1305 Decrypt** | p50 / p95 / p99 | **0.073 ms** / 0.075 ms / 0.077 ms |
| **Peak Linux Memory Footprint** | VmHWM | **< 5 MB** (4,896 kB) |

*Performance Note:* Claim ticket issuance and database ledger inserts were optimized using atomic multi-row batch inserts (`UNNEST` / batch bind in `crates/store`), eliminating single-row round-trip serialization bottlenecks.

---

## 6. End-to-End Workflow Demonstration

To run the complete 10-step simulated workflow exercising real Zcash key derivation, Orchard unified addresses, and token lifecycles:

```bash
./scripts/demo.sh
```

Or via Cargo:

```bash
cargo run --package zsplit-api --example workflow_demo
```

### Demonstration Steps Validated

1. Organizer creates room with 50 claims (0.001 ZEC each).
2. Coordinator assigns isolated deposit address and validates required funding.
3. Backend issues 50 cryptographically unique bearer tokens with persistent HMAC digests.
4. Recipient opens QR claim link with token safely isolated in fragment (`#t=...`).
5. Recipient client derives ephemeral Orchard keypair locally without transmitting private keys.
6. Backend verifies bearer token in constant time and queues exactly one payout.
7. Wallet worker prepares shielded payout transaction and records broadcast hash.
8. Wallet synchronizer tracks block confirmations to `FundedConfirmed`.
9. Recipient sweeps funds locally to private personal address unknown to the backend.
10. Organizer dashboard reflects updated room state (`funded: 1, reserved: 49, expired: 0`).

---

## 7. Known Limitations & Trust Model

1. **Coordinator Custody:** The coordinator holds funds in custodial Zcash accounts from deposit until redemption. Organizers must be trusted not to abscond with unredeemed funds before expiration.
2. **Bearer Token Protection:** Anyone holding the `#t=...` token link can redeem it. If a bearer link is forwarded or intercepted, it can be redeemed by the holder.
3. **Network & Pool Compatibility:** Pinned to Zcash testnet with live consensus branch validation. Post-NU6.3 transactions target the Ironwood shielded pool with backwards compatibility for Orchard notes.
4. **Sweep Verification vs Self-Report:** The backend can independently verify funding to the ephemeral address via on-chain sync. It **cannot** independently verify the subsequent sweep to a personal address without violating recipient viewing privacy; sweep status relies on recipient client self-reports.
