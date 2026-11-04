# Zool — Z-Split

**Shielded group payouts through one-time claim links.**

Zool is the web application for Z-Split, a Zcash payout coordinator for hackathon prizes, community rewards, contributor payments, and event distributions. An organizer funds a room, issues tickets for fixed amounts, and distributes links or QR codes. Recipients redeem into a separate shielded claim wallet, keeping their spending keys and eventual personal sweep destinations off the coordinator.

The project combines a **SvelteKit frontend**, a **Rust/Axum backend**, **PostgreSQL application accounting**, and an **official native Zcash wallet**. It uses a **reserve-first, fund-on-redemption** model: issuing a ticket reserves funds; redeeming it queues a real shielded payment.

> **Current scope:** a testnet prototype with genuine confirmed payouts. The coordinator is custodial until funding reaches the recipient wallet. The integrated browser wallet, production authentication, and a security-reviewed deployment remain unfinished. Gift cards and sample claim flows are demonstrations. See [implementation status](#implementation-status) before using the app.

## Contents

- [What the project solves](#what-the-project-solves)
- [Implementation status](#implementation-status)
- [Technology stack](#technology-stack)
- [Architecture and design decisions](#architecture-and-design-decisions)
- [Payment lifecycle and duplicate prevention](#payment-lifecycle-and-duplicate-prevention)
- [Privacy, custody, and security](#privacy-custody-and-security)
- [Repository structure](#repository-structure)
- [Local setup](#local-setup)
- [Configuration reference](#configuration-reference)
- [Using the application](#using-the-application)
- [API and OpenAPI](#api-and-openapi)
- [Email, QR codes, and exports](#email-qr-codes-and-exports)
- [Testing and verification](#testing-and-verification)
- [Performance and optimization](#performance-and-optimization)
- [Deployment and operations](#deployment-and-operations)
- [Troubleshooting](#troubleshooting)
- [Limitations and next steps](#limitations-and-next-steps)
- [Documentation, provenance, and licensing](#documentation-provenance-and-licensing)

## What the project solves

Traditional group crypto payments often start with a spreadsheet linking names or emails to personal wallet addresses. Shielded transactions cannot erase that off-chain record. Z-Split replaces manual destination collection with bearer tickets and recipient-driven redemption.

The organizer defines amounts, including tiered prizes, and receives shareable tickets. A recipient supplies only a fresh shielded receiving address for a separate claim wallet. The recipient's eventual transfer to a personal wallet happens locally in their own wallet software.

**“No personal address collection” has a precise meaning here:** the backend still receives the claim wallet's temporary receiving address so it can pay it. That address is encrypted in the application queue, and native sender-wallet history can retain output metadata. The application cannot determine whether an address supplied by a user is actually separate from their personal wallet.

## Implementation status

| Capability                                    | Current implementation                                                                                                              |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Rooms, amounts, reservations, and fee budgets | Rust API with PostgreSQL transactions and migrations                                                                                |
| One-time tickets                              | Secure 256-bit bearer tokens; keyed digests in PostgreSQL                                                                           |
| Redemption and payout queue                   | Atomic state transition, stable payout intent, worker leases and recovery                                                           |
| Native Zcash integration                      | Official wallet SDK, SQLite persistence, lightwalletd synchronization, transaction construction, proving, signing, and broadcasting |
| Real chain evidence                           | Three existing payouts confirmed on testnet; IDs and observations in the [recovery report](zsplit-backend/docs/payout-recovery.md)  |
| Claim receipt                                 | Automatic status polling, loading indicator, transaction ID, confirmations, visible errors, and manual refresh                      |
| Organizer reporting                           | Authenticated summary and SSE API; web dashboard integration                                                                        |
| Distribution                                  | Per-ticket links, QR sheet, vector PDF export, printing, optional Resend email                                                      |
| Recipient wallet                              | Current UI accepts an externally created shielded claim-wallet address and a backup attestation; no embedded WebZjs wallet          |
| Personal sweep                                | Performed in the recipient's external wallet; backend accepts an optional, unverified completion report                             |
| Expiry and withdrawals                        | Unredeemed reservation expiry and authenticated withdrawal API; not a clawback of funded notes                                      |
| FROST                                         | Experimental Ed25519 threshold-signature demonstration; not enforced Zcash treasury signing                                         |
| Gift cards                                    | Demo UI only; no Bitrefill order, invoice payment, or fulfillment integration                                                       |
| Mainnet / regtest                             | Runtime currently permits testnet only                                                                                              |

The original [product brief](z-split-project-info.md) and [backend specification](Z_SPLIT_BACKEND_ARCHITECTURE.md) describe both goals and proposed components. This README describes the current implementation; their unchecked goals are not evidence that a feature works.

## Technology stack

Versions below come from the repository manifests. `package-lock.json` and `zsplit-backend/Cargo.lock` record resolved dependencies; use those lockfiles for reproducible installs.

### Web application

| Technology                   | Manifest version   | Role and reason                                                                      |
| ---------------------------- | ------------------ | ------------------------------------------------------------------------------------ |
| Node.js                      | ≥22.17             | JavaScript tooling and SvelteKit server runtime                                      |
| Svelte                       | 5.57.x             | Reactive components and shared application state using runes                         |
| SvelteKit                    | 3.1.x              | File-based routes, server rendering, navigation, and the server-only email endpoint  |
| TypeScript                   | 6.x                | Typed frontend models, API contracts, and configuration                              |
| Vite                         | 8.3.x              | Development server, builds, and local Rust API proxy                                 |
| Tailwind CSS / Vite plugin   | 4.x                | Styling tools alongside the application's custom CSS                                 |
| Motion                       | 12.x               | Interactive illustration springs and UI motion, with reduced-motion handling         |
| `qrcode`                     | 1.5.x              | Ticket QR generation and email PNG rendering                                         |
| Custom SVG and PDF code      | Repository source  | Product artwork and vector ticket-sheet export without a hosted rendering service    |
| Anton, DM Sans, Patrick Hand | Self-hosted WOFF2  | Display, interface, and handwritten typography; font licenses are in `static/fonts/` |
| Resend HTTP API              | Server integration | Optional email delivery; credentials stay in the SvelteKit server                    |
| `adapter-node`               | 6.x                | Standalone Node server for the GCP deployment                                        |

### Backend and data

| Technology                   | Manifest version                  | Role and reason                                                                      |
| ---------------------------- | --------------------------------- | ------------------------------------------------------------------------------------ |
| Rust                         | Edition 2024; minimum 1.94        | Backend, domain types, accounting, and native wallet integration                     |
| Tokio                        | 1.x                               | Async HTTP/network execution, task coordination, and shutdown signals                |
| Axum                         | 0.8.x                             | Typed HTTP routing and request handling                                              |
| Utoipa / Swagger UI          | 6.0.0 / 10.0.2, exact pins        | OpenAPI generation and interactive API documentation                                 |
| SQLx                         | 0.9.0, exact pin                  | PostgreSQL queries, transactions, pooling, and migrations                            |
| PostgreSQL                   | 16 in Compose/CI                  | Application metadata, liabilities, outbox jobs, audit events, and transaction status |
| SQLite / Rusqlite            | Rusqlite 0.37 with bundled SQLite | Native wallet storage, compact-block cache support, and the local intent journal     |
| Tonic                        | 0.14.x                            | Compatible lightwalletd gRPC transport with TLS/WebPKI roots                         |
| Tower / Tower HTTP           | 0.5.x / 0.6.x                     | HTTP middleware including CORS, limits, and headers                                  |
| Serde / Serde JSON           | 1.x                               | Structured request, response, and persistence serialization                          |
| UUID / Chrono                | 1.x / 0.4.x                       | Identifiers, idempotency keys, timestamps, and expiration                            |
| Tracing / tracing-subscriber | 0.1.x / 0.3.x                     | Structured JSON logs and log filtering                                               |
| Thiserror / Anyhow           | 2.x / 1.x                         | Domain errors and startup/operator error context                                     |
| Dotenvy                      | 0.15.7                            | Loading local backend configuration without overriding exported variables            |

### Official Zcash SDK family

These dependencies are pinned together in the [workspace manifest](zsplit-backend/Cargo.toml). They implement wallet and consensus behavior; the project does not implement its own Zcash cryptography.

| Crate                  | Exact version  | Responsibility                                                                                |
| ---------------------- | -------------- | --------------------------------------------------------------------------------------------- |
| `zcash_client_backend` | 0.25.0         | Wallet synchronization, proposals, input selection, transaction creation, lightwalletd client |
| `zcash_client_sqlite`  | 0.23.0         | Official native wallet persistence                                                            |
| `zcash_keys`           | 0.17.0         | Unified key and address operations                                                            |
| `zcash_address`        | 0.14.0         | Address decoding and network validation                                                       |
| `zcash_protocol`       | 0.11.0         | Network and consensus types                                                                   |
| `zcash_primitives`     | 0.31.0         | Transaction primitives                                                                        |
| `zcash_proofs`         | 0.31.0         | Native proving components and public parameter verification                                   |
| `orchard`              | 0.16.0         | Shielded pool primitives used through SDK integration                                         |
| `zip32` / `zip321`     | 0.3.0 / 0.10.0 | Key derivation support and payment-request types                                              |

The adapter validates the upstream network and consensus branch and selects the active output pool using the pinned protocol rules. It distinguishes Orchard-capable unified receivers from the active chain value pool; the current code includes Ironwood handling. Do not assume an older Orchard-only browser wallet is compatible with the configured testnet. ZIP-321 describes payment requests; it does not make an application ticket redeemable by an arbitrary wallet.

### Security and development tools

| Technology                                      | Use                                                                                               |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `rand` / OS randomness, HMAC, SHA-256, `subtle` | Ticket generation, keyed token digests, and constant-time verification                            |
| ChaCha20-Poly1305                               | Authenticated encryption of queued ephemeral receivers, bound to the claim ID                     |
| `zeroize` / `secrecy`                           | Sensitive-value handling and memory cleanup where supported                                       |
| Base64 / Hex                                    | Canonical token and configuration encodings                                                       |
| `async-trait`, `futures-util`, `tokio-stream`   | Async engine interfaces, stream utilities, and SSE support                                        |
| Rustls / Ring / WebPKI                          | TLS verification and crypto-provider support through the configured SQLx/Tonic transport features |
| `tempfile`                                      | Temporary native-wallet/test resources                                                            |
| `frost-ed25519` / `frost-core` 3.0.0            | Experimental dealer-generated threshold authorization demonstration                               |
| Node test runner                                | Frontend model, API, email, and UUID unit tests                                                   |
| Playwright                                      | Browser workflows, mobile receipt behavior, and API failure handling                              |
| Rustfmt / Clippy / Cargo tests                  | Formatting, linting, unit and integration checks                                                  |
| Prettier / Svelte Check                         | Web formatting and TypeScript/Svelte diagnostics                                                  |
| Docker / Docker Compose                         | PostgreSQL development service and reference backend container                                    |
| GitHub Actions                                  | Backend format, lint, unit tests, PostgreSQL tests, and release microbenchmarks                   |
| Custom `zsplit-bench`                           | Percentiles for local crypto and optional PostgreSQL operations; Linux peak RSS observation       |
| Python 3 standard library                       | Downloading and integrity-checking public proving parameter files                                 |

No Redis, Kafka, Kubernetes, hosted wallet, or blockchain simulation service is required by the current application.

## Architecture and design decisions

```mermaid
flowchart LR
    O[Organizer browser] --> WEB[SvelteKit web app]
    R[Recipient browser] --> WEB
    WEB --> API[Axum API]
    WEB --> MAIL[Resend email API]
    API --> PG[(PostgreSQL)]
    API --> WORK[Serialized wallet worker]
    WORK --> PG
    WORK --> SDK[Official native Zcash SDK]
    SDK --> DB[(Wallet SQLite and intent journal)]
    SDK --> LWD[lightwalletd gRPC]
    LWD --> CHAIN[Zcash testnet]
    R --> WALLET[Recipient external claim wallet]
    WALLET --> PERSONAL[Local sweep to personal wallet]
```

1. **Reserve first, fund on redemption.** Tickets represent reserved coordinator funds, not spending keys or pre-funded notes. This allows unused reservations to expire without pretending a website timer can revoke a Zcash key.
2. **Separate accounting from wallet state.** PostgreSQL owns room obligations and payout jobs. The official SQLite wallet owns notes, witnesses, and native transaction history. Application SQL does not substitute for chain synchronization.
3. **One combined API/worker process per wallet.** An exclusive wallet-directory lock and serialized native writer prevent competing coordinators from selecting the same notes. Horizontal API replication is not supported against the same wallet directory.
4. **Keep proving outside HTTP handlers.** Wallet work runs through bounded blocking execution (`spawn_blocking` and a semaphore). HTTP redemption accepts a durable job rather than waiting for a proof or a block.
5. **Use an outbox with explicit uncertainty.** Database idempotency alone cannot guarantee exactly-once blockchain effects. A native intent journal, persisted signed bytes, and reconciliation bridge the PostgreSQL/wallet/network boundaries.
6. **Use fragment bearer tickets.** Links have the form `/claim/{claim_id}#t={token}`. Browsers omit the fragment from HTTP URL requests; frontend code supplies the ticket through an `Authorization` header for claim API calls.
7. **Count funding separately from sweeping.** `funded_confirmed` is backed by native chain reconciliation. `sweep_reported` is a user statement, not an independently verified transfer.
8. **Keep optional integrations separate.** Email runs in SvelteKit with server-only credentials. Gift-card and FROST experiments do not participate in the production payout authorization path.

SQLx 0.9 and the pinned Zcash family were selected together to resolve native SQLite linkage compatibility. Changing one wallet crate independently can break the dependency graph or protocol behavior. See [ADR 001](zsplit-backend/docs/adr-001-redemption-model.md) for the recovery tradeoff.

## Payment lifecycle and duplicate prevention

```text
reserved → funding_queued → preparing → funding_broadcast
                                           ↓
                                  funded_unconfirmed → funded_confirmed

Unused reserved ticket → expired → reservation released
Uncertain construction/broadcast → reconciliation or manual_review
Reorganization → reorged → reconciliation
Sweep report → separate unverified flag
```

A room reserves payout amounts plus a per-ticket fee budget. API amounts are integer **zatoshis**: `1 ZEC = 100,000,000 zat`. The frontend converts values for display. SDK transaction selection and fee rules determine actual construction requirements; a configured budget is not a guarantee that an arbitrary payout is currently spendable.

On redemption, row locks and unique constraints atomically consume one ticket and insert at most one payout intent. Retries must keep the same receiver and idempotency UUID. Worker leasing avoids two workers taking the same queue row, and a native journal marker is written before transaction construction. Signed transaction bytes are persisted before broadcast.

A timeout permits reconciliation or rebroadcast of the **same bytes**. It does not authorize building a replacement transaction. An unresolved construction remains quarantined with its reservation intact. A temporary insufficient-spendable-balance failure can be deferred only when the native journal proves construction never began; this covers waiting for confirmed change after an earlier payout.

The receipt polls pending claims, shows preparing/broadcast/mined/confirmed states, and provides retry after status errors. Manual review is displayed as paused. Neither an HTTP 202 nor a successful broadcast response is payment confirmation.

## Privacy, custody, and security

### What stays private

Recipient spending keys, recovery seeds, and personal sweep destinations are outside the payout API contract. The organizer dashboard does not expose a recipient-address list. PostgreSQL stores keyed ticket digests rather than raw tickets, and pending claim receivers are encrypted with claim-bound authenticated encryption.

### What is visible

- The coordinator sees room budgets, ticket amounts, claim activity, and temporary claim receivers during processing. Native wallet files can retain sent-output metadata.
- The organizer browser holds issued bearer links for export and distribution; those links are persisted in browser storage. Browser compromise can expose them.
- Optional email distribution exposes recipient email addresses and assigned claim links to the SvelteKit mail handler and Resend. This is less private than handing out QR tickets in person.
- The server and network intermediaries can observe timing and connection metadata. lightwalletd sees the coordinator connection and transaction broadcasts. Shielding does not eliminate all timing correlation.
- A fragment protects the token from ordinary request URL logging, but not from malicious JavaScript, browser extensions, screenshots, forwarded messages, or an email provider carrying the link.

### What users must trust

The coordinator holds the seed and controls deposited funds until payout. Organizers can retain or copy tickets, and anyone with a valid ticket can attempt redemption first. The app is **custodial**, not trustless or non-custodial.

Recipients must use a separate claim wallet, save its recovery backup, and retain it until funds are received and swept. The current checkbox records an attestation; the app cannot prove that an external wallet backup exists. Lost wallet keys can make received funds unrecoverable. The operator must protect PostgreSQL, native wallet data, coordinator seed, token HMAC key, and receiver encryption key.

The API includes organizer ownership checks, input validation, CORS restrictions, request limits, rate limiting, and structured errors/logging. These are prototype safeguards, not an audit certification. The organizer mechanism is a configured bearer token with an `/organizer` sign-in page, not a multi-user login system. The known fallback token is restricted to development builds.

See the [threat model](zsplit-backend/docs/privacy-and-threat-model.md) and [browser compatibility requirements](zsplit-backend/docs/browser-wallet-compatibility.md). Some historical pitch documents use stronger privacy or FROST language than the implemented guarantees above.

## Repository structure

```text
.
├── src/
│   ├── lib/screens/             # Organizer, claim, dashboard, and marketing UI
│   ├── lib/components/          # Shared controls, QR rendering, and artwork
│   ├── lib/server/claim-email.ts # Email validation, rendering, and provider dispatch
│   ├── lib/api.ts               # Rust API client
│   ├── lib/state.svelte.ts      # Browser state and persistence
│   ├── lib/model.ts             # Room models, calculations, and storage validation
│   ├── lib/pdf.ts               # Vector ticket PDF generation
│   ├── lib/uuid.ts              # Secure UUID compatibility helper
│   └── routes/                 # SvelteKit pages and /resend-email server route
├── static/fonts/               # Self-hosted fonts and licenses
├── tests/                      # Node unit tests and Playwright browser tests
├── zsplit-backend/
│   ├── apps/api/               # Axum executable, API handlers, OpenAPI, examples
│   ├── apps/worker/            # Queue dispatch, reconciliation, recovery example
│   ├── crates/domain/         # Shared models, checked amounts, errors, engine trait
│   ├── crates/store/          # PostgreSQL accounting and transactional outbox
│   ├── crates/claims/         # Ticket generation, validation, receiver encryption
│   ├── crates/zcash/          # Official native SDK integration and wallet tools
│   ├── crates/frost/          # Experimental threshold-signature demonstration
│   ├── migrations/            # SQLx database schema migrations
│   ├── tests/                 # PostgreSQL and recovery/concurrency fixtures
│   ├── benches/               # Custom benchmark binary
│   ├── scripts/               # Verified public proving-parameter downloader
│   └── docs/                  # Contracts, OpenAPI snapshot, privacy and reports
├── .github/workflows/ci.yml    # Backend CI
├── .env.example               # SvelteKit server/mail and Vite proxy settings
└── zsplit-backend/.env.example # Backend, database, and native wallet settings
```

## Local setup

### Prerequisites

Install Node.js ≥22.17 with npm, Rust ≥1.94 with Cargo/Rustfmt/Clippy, Python 3, and PostgreSQL 16 or Docker Compose. Native builds also need a C/C++ build toolchain, `pkg-config`, OpenSSL development libraries, SQLite development libraries, and `protoc`. The provided Dockerfile and CI workflow list Debian/Ubuntu package names.

Use testnet funds only. The API currently rejects other networks. A compatible lightwalletd endpoint and outbound TLS access are required; the template contains the upstream used during development, whose availability must be checked at runtime.

### 1. Install and configure

From the repository root, for a fresh checkout:

```sh
npm ci
cp .env.example .env
cp zsplit-backend/.env.example zsplit-backend/.env
```

Do not overwrite an existing configuration. Edit both files before startup. Generate independent backend secrets, for example with Python's `secrets.token_hex(32)`, and replace the template token and two 32-byte hex keys. Match the organizer bearer token in both environment files. Keep `.env`, wallet files, seeds, tickets, and database backups out of source control.

### 2. Start PostgreSQL

```sh
cd zsplit-backend
docker compose up -d postgres
```

This starts only the development database. The template `DATABASE_URL` matches the Compose database's initial credentials. If a database volume already exists, changing environment files does not change its password or reset its data. Use the actual credentials and preserve the configured organizer ID on an existing installation.

### 3. Initialize the coordinator and proving parameters

From `zsplit-backend/`, on the first installation only:

```sh
cargo run --locked --package zsplit-zcash --example wallet -- seed
python3 scripts/download-proving-params.py
```

The seed command uses the default `./data/coordinator.seed` path unless `ZCASH_SEED_FILE` is exported. Unlike the API, this example does not automatically load `.env`; export any custom path first. It creates a protected seed file, not a printable recovery phrase. Back up the file securely before accepting deposits and never replace it as a troubleshooting step.

The downloader verifies exact sizes and BLAKE2b hashes for the public Sapling parameter files expected by the pinned prover. The current native adapter requires these files even for its Orchard/Ironwood payout path. Ensure both parameter paths in the backend `.env` match the downloaded files.

### 4. Start the Rust API and worker

In the same backend directory:

```sh
cargo run --locked --package zsplit-api
```

The API loads `zsplit-backend/.env`; exported variables take precedence. SQLx migrations run at startup. Default listener: `127.0.0.1:8080`. The same process runs wallet sync and payout dispatch. Initial wallet initialization/sync can take time.

In another terminal:

```sh
curl --fail http://127.0.0.1:8080/healthz
curl --fail http://127.0.0.1:8080/readyz
```

Open `http://127.0.0.1:8080/docs` when `ENABLE_DOCS=true`.

### 5. Start the web application

From the repository root:

```sh
npm run dev
```

Open the URL Vite prints, normally `http://localhost:5173`. Vite proxies `/api` and `/healthz` to port 8080. Configure `ZSPLIT_API_URL` and restart Vite if the Rust listener differs. If Vite selects another port, align backend `ALLOWED_ORIGIN` and `PUBLIC_CLAIM_ORIGIN` with the actual browser origin.

For a custom organizer token, open `/organizer` and sign in with the configured token. For local diagnostics, the same browser token can also be set on the app origin:

```js
localStorage.setItem('zsplit-auth-token', 'YOUR_CONFIGURED_ORGANIZER_TOKEN');
location.reload();
```

Treat this as a local development mechanism. Recipients authenticate with their individual ticket, not the organizer token.

## Configuration reference

### Root `.env` — SvelteKit and development proxy

| Variable              | Purpose                                                                           |
| --------------------- | --------------------------------------------------------------------------------- |
| `RESEND_API_KEY`      | Server-only email provider key; optional if not emailing tickets                  |
| `RESEND_FROM_EMAIL`   | Sender on a domain verified by your Resend account                                |
| `ORGANIZER_API_TOKEN` | Authorizes the SvelteKit email endpoint; match the Rust API and organizer browser |
| `ZSPLIT_API_URL`      | Optional Vite proxy target; defaults to `http://127.0.0.1:8080`                   |

### Backend `.env`

| Variable                                         | Purpose                                                                                                          |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `DATABASE_URL`                                   | PostgreSQL connection string                                                                                     |
| `ORGANIZER_ID`                                   | Configured organizer UUID; preserve it for an existing database                                                  |
| `ORGANIZER_API_TOKEN`                            | Organizer API bearer secret, at least 32 characters                                                              |
| `CLAIM_TOKEN_KEY`                                | 32-byte HMAC key encoded as 64 hex characters                                                                    |
| `RECEIVER_ENCRYPTION_KEY`                        | Independent 32-byte encryption key encoded as 64 hex characters                                                  |
| `PORT` / `BIND_ADDR`                             | Port defaults to 8080; explicit socket address overrides it                                                      |
| `ALLOWED_ORIGIN`                                 | Allowed frontend origin for CORS                                                                                 |
| `PUBLIC_CLAIM_ORIGIN`                            | Public origin embedded in ticket links; never use localhost for recipients on other devices                      |
| `ENABLE_DOCS`                                    | Enables interactive Swagger UI when `true`                                                                       |
| `ENABLE_DEVELOPMENT_ENDPOINTS`                   | Enables synthetic funding and experimental FROST routes only when `true`; default and GCP deployment are `false` |
| `ZCASH_NETWORK`                                  | `testnet`; other networks are disabled by this build                                                             |
| `ZCASH_CONFIRMATIONS`                            | Funding confirmation depth; template uses 3                                                                      |
| `ZCASH_DATA_DIR`                                 | Persistent private native wallet directory                                                                       |
| `ZCASH_SEED_FILE`                                | Protected coordinator seed file                                                                                  |
| `LIGHTWALLETD_URL`                               | Compatible TLS gRPC upstream                                                                                     |
| `ZCASH_BIRTHDAY_HEIGHT`                          | Optional scan birthday; an incorrect late birthday can miss funds                                                |
| `SAPLING_SPEND_PARAMS` / `SAPLING_OUTPUT_PARAMS` | Paths to the verified public proving parameter files                                                             |
| `RUST_LOG`                                       | Standard tracing filter; export before startup for logging initialization                                        |

Never prefix provider keys or backend secrets with `PUBLIC_` or `VITE_`. The two `.env` files serve different runtimes. Paths are relative to the process working directory; the instructions above run the Rust process from `zsplit-backend/`.

## Using the application

1. Open `/room/new` and configure the room, expiry, and payout amounts or prize tiers.
2. Generate the coordinator deposit address. Send actual shielded **testnet** funds covering payouts and fees. Wait for native synchronization and the configured confirmations.
3. Mint tickets once the room has sufficient spendable funds. Export or securely save the tickets: the backend cannot recover plaintext tokens from their digests.
4. Share individual links, print a QR sheet, export the PDF, or optionally email assigned tickets.
5. The recipient opens the complete link, including `#t=...`, and creates a separate compatible shielded wallet in external wallet software. Save its recovery backup.
6. Enter that claim wallet's receiving address and explicitly attest that the wallet backup is saved. Redeem once; retry an uncertain request through the existing page instead of creating a new payment.
7. Watch the receipt progress from queued to preparing, broadcast, mined, and confirmed. Independently sync the recipient wallet to detect the funds.
8. Sweep locally from the claim wallet to the personal wallet. Do not enter the personal sweep address into Zool. An optional completion report remains unverified.
9. The organizer checks funded, reserved, pending, expired, and manual-review counts. Unused expired reservations can be released; withdrawing remaining coordinator funds is a separate authorized operation.

| Web route                               | Purpose                                     |
| --------------------------------------- | ------------------------------------------- |
| `/`                                     | Product landing page                        |
| `/rooms`                                | Current room overview                       |
| `/room/new`                             | Room setup                                  |
| `/room/fund`                            | Deposit instructions and funding status     |
| `/room/claims`                          | Ticket management, export, and distribution |
| `/room/live`                            | Organizer dashboard                         |
| `/claim`                                | Paste a claim link                          |
| `/claim/{claim_id}`                     | Ticket introduction                         |
| `/claim/{claim_id}/payout`              | Claim-wallet receiver entry                 |
| `/claim/{claim_id}/done`                | Live payment receipt                        |
| `/how-it-works`, `/privacy`             | Product guidance                            |
| `/claim/demo`, `/claim/sample-{number}` | Local sample flows; no chain evidence       |

## API and OpenAPI

Base path: `/api/v1`. Organizer operations use `Authorization: Bearer <organizer-token>` and enforce room ownership. Claim operations use `Authorization: Bearer <claim-ticket>`. Amounts are integer zatoshis and network fields must match the running backend.

| Method | Route                                    | Access / purpose                                          |
| ------ | ---------------------------------------- | --------------------------------------------------------- |
| GET    | `/healthz`, `/readyz`                    | Public liveness and dependency readiness                  |
| GET    | `/api/v1/meta`                           | Public network/policy metadata                            |
| POST   | `/api/v1/rooms`                          | Organizer creates room                                    |
| GET    | `/api/v1/rooms/{room_id}`                | Organizer reads room                                      |
| GET    | `/api/v1/rooms/{room_id}/deposit`        | Organizer reads deposit address and balances              |
| POST   | `/api/v1/rooms/{room_id}/claims`         | Organizer issues tickets once                             |
| GET    | `/api/v1/rooms/{room_id}/summary`        | Organizer counts and accounting                           |
| GET    | `/api/v1/rooms/{room_id}/events`         | Authenticated SSE updates                                 |
| POST   | `/api/v1/rooms/{room_id}/withdrawals`    | Organizer queues unreserved-fund withdrawal               |
| GET    | `/api/v1/claims/{claim_id}`              | Ticket-authenticated metadata                             |
| POST   | `/api/v1/claims/{claim_id}/redeem`       | Ticket-authenticated redemption                           |
| GET    | `/api/v1/claims/{claim_id}/status`       | Funding status and confirmations                          |
| POST   | `/api/v1/claims/{claim_id}/sweep-report` | Optional unverified completion report                     |
| POST   | `/resend-email`                          | Separate SvelteKit organizer-authenticated email endpoint |

Example redemption payload, using a real separate claim-wallet address:

```json
{
  "ephemeral_receiver": "<compatible testnet shielded unified address>",
  "idempotency_key": "6c87fb7c-c78a-4b86-8e13-52c98aad4b63",
  "client_network": "testnet",
  "wallet_persisted": true
}
```

A successful redemption returns an accepted queue operation, not synchronous confirmation. Never set `wallet_persisted` merely to bypass backup requirements. Errors use structured codes/messages and correlation IDs; callers must handle conflicts, rate limits, unavailable sync, and paused payout states.

The running schema is available at `/openapi.json`; interactive docs are at `/docs`. A checked-in snapshot lives at [docs/openapi.json](zsplit-backend/docs/openapi.json). Regenerate the Rust schema without opening the wallet:

```sh
cd zsplit-backend
cargo run --locked --package zsplit-api --example export-openapi > docs/openapi.json
```

**Development-only surface:** the router registers, only when `ENABLE_DEVELOPMENT_ENDPOINTS=true`, `/api/v1/rooms/{room_id}/dev-confirm`, which writes synthetic application funding state, and experimental `/api/v1/frost/setup` and `/api/v1/frost/sign` routes. They are not evidence of blockchain funding or secure treasury governance. The FROST endpoints currently generate/handle signing shares on the server and do not enforce payout authorization. They are disabled by default and blocked by the GCP proxy; not every development route is covered by the generated OpenAPI contract.

## Email, QR codes, and exports

Email distribution is optional. Set a Resend key and a sender such as `Zool <claims@your-verified-domain.com>` in the root `.env`, then restart the SvelteKit server. The default `onboarding@resend.dev` sender is suitable only for provider-restricted account-owner testing.

Each message contains only its assigned ticket and amount, a claim link, and a PNG QR image referenced through a Content-ID attachment. Messages use individual paced provider requests, because the selected inline attachment flow is not sent through the batch endpoint. Stable per-message idempotency keys reduce duplicates within the provider's retry window. The dialog distinguishes rejected, partially accepted, and accepted requests; **provider acceptance does not prove inbox delivery**.

The QR encodes an application claim URL, not a Zcash private key or a ZIP-321 redemption protocol. Preserve the fragment when sharing. PNG quiet zones help scanners; email clients can still block images, so use the text link if the QR is hidden. For mobile recipients, configure an origin reachable from their device over HTTPS instead of `localhost`.

The ticket sheet offers an actual vector PDF export and browser printing. Exported PDFs, screenshots, email messages, and browser ticket storage contain bearer authority: distribute and retain them accordingly.

## Testing and verification

From the repository root:

```sh
npm run check
npm test
npm run build
npx playwright install chromium
npm run dev -- --port 5174
```

With that dev server running, use another terminal:

```sh
npx playwright test
```

`playwright.config.ts` targets port 5174. For only the current payout/claim regressions:

```sh
npx playwright test tests/browser/claim-status.spec.ts tests/browser/redemption.spec.ts
```

From `zsplit-backend/`:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

Run database-dependent tests explicitly against a dedicated disposable database:

```sh
DATABASE_URL=postgres://TEST_USER:TEST_PASSWORD@localhost:5432/zsplit_test \
  cargo test --locked --package zsplit-integration --test postgres -- --ignored
```

The fixture user needs permission to create/drop isolated schemas. These tests exercise real PostgreSQL transactions with controlled engine outcomes; they do not create live chain reorganizations. Coverage includes 100 competing redemptions producing one intent, atomic issuance, queue leases, interrupted construction, journal-guarded recovery, expiry, reorg accounting, and declining wallet balances.

### Last recorded verification — 4 November 2026

| Check                                       | Recorded result                                                                           |
| ------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Rust format, strict Clippy, workspace tests | Passed                                                                                    |
| Explicit PostgreSQL integration suite       | 12 passed                                                                                 |
| Svelte/TypeScript diagnostics               | Zero errors and warnings                                                                  |
| Frontend unit tests                         | 14 passed                                                                                 |
| Frontend production build                   | Passed; Node adapter configured for deployment                                            |
| Full Playwright suite                       | 13 passed, 5 failed                                                                       |
| New payout-status tests                     | Both passed, including polling, confirmations, paused state, and refresh failure recovery |
| Genuine testnet payouts                     | Three recorded as confirmed by native reconciliation                                      |

The five remaining browser failures concern old landing/demo selectors and controls (`.scene-toy`, older landing links, and `Fund and make links`). Do not describe the entire suite as green. Browser payment fixtures use synthetic statuses/IDs and do not prove blockchain success. See the [latest incident report](zsplit-backend/docs/payout-recovery.md), which supersedes older result counts in historical reports.

Dependency/security audits are separate checks, not a CI guarantee. Operators can install and run `cargo audit`/`cargo deny` and run `npm audit`, reviewing findings against the actual lockfiles and deployment. No new audit, regtest run, or live chain-reorganization experiment was performed for the payout incident or this documentation update.

## Performance and optimization

The custom benchmark emits JSON with p50/p95/p99, minima/maxima, raw samples, and Linux `VmHWM` for the benchmark process. Run from `zsplit-backend/`:

```sh
cargo run --locked --release --package zsplit-integration --bin zsplit-bench
```

Setting `DATABASE_URL` to a disposable test database enables 50-ticket issuance, enqueue, lease/quarantine, and summary query benchmarks. `BENCH_ITERATIONS` controls fixture iterations; in-memory crypto uses ten times that count. The default is 20 fixture iterations, or 200 crypto samples.

Historical crypto measurements in the [benchmark report](zsplit-backend/docs/benchmark-report.md) include approximately 0.034 ms median to generate 50 tokens and 0.015 ms to verify 50 tokens. These are **local microbenchmarks**, not HTTP request latency or shielded proof throughput. Reported benchmark-process memory is not the wallet server's peak memory.

Implemented performance choices include batched claim inserts, indexed/leased queue access, serialized wallet state, and bounded proof work outside the HTTP executor. The existing reports do not establish a complete reproducible before/after optimization study. API load percentiles, real proof CPU/RAM, cold/warm wallet sync, queue throughput under live funding, and full-server memory still need fresh measurements on the intended deployment. Chain confirmation time must be reported separately from request latency.

## Deployment and operations

The [GCP deployment guide](deploy/gcp/README.md) describes the Compute Engine deployment, HTTPS routing, persistent volumes, organizer access, and recovery. The GCP Compose file disables development endpoints and uses the Node adapter.

The supported development path is PostgreSQL in Compose plus the API and web app run locally. `zsplit-backend/Dockerfile` defines a multi-stage Rust build and non-root runtime; `docker-compose.yml` is a **development reference**, containing known example credentials.

The reference API container is not a fully provisioned wallet deployment: the operator must initialize and mount the seed, wallet directory, and verified proving files and set the parameter paths. The runtime image does not include the wallet seed-generation example or the Python bootstrap script. Do not assume `docker compose up` alone can create and fund a working coordinator. Container builds and fresh-host deployment were not revalidated for this README update.

For a deployed web app, choose a SvelteKit adapter for the host, provide a server runtime for `/resend-email`, configure HTTPS and a production reverse proxy for `/api`, and preserve the public claim origin. Vite's development proxy does not configure production routing. A purely static host cannot serve the email handler by itself.

Before accepting funds, keep development funding and FROST routes disabled, replace default credentials, establish proper organizer sessions, review browser secret storage, and restrict access to operational documentation and Swagger as appropriate. Serve claim pages without third-party tracking and review CSP, referrer policy, proxy logging, and TLS configuration for the deployment.

Back up PostgreSQL, the coordinator seed, native wallet state, native intent journal, and application secrets together with a tested recovery procedure. Never delete wallet files or reset a claim to recover a timeout. Only one process may own a wallet directory. SIGINT/SIGTERM initiates graceful shutdown and waits for worker recovery state rather than blindly aborting proof work.

For a paused payout, follow [payout recovery](zsplit-backend/docs/payout-recovery.md). The operator recovery example refuses any native construction marker or signed transaction and preserves existing intent IDs/reservations. A review state is a safety boundary, not a failed ticket to redeem again.

## Troubleshooting

| Symptom                                    | Check / action                                                                                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Vite `ECONNREFUSED` for `/api/v1/rooms`    | Start the Rust API, inspect startup errors, and match its listener to `ZSPLIT_API_URL`; default is 8080, not 8000                                  |
| PostgreSQL auth or organizer setup failure | Use existing database credentials and organizer UUID; copying `.env.example` does not reset a volume                                               |
| Wallet locked                              | Stop the other API/operator process gracefully; do not delete the journal or lock file to bypass ownership                                         |
| Funding remains unavailable                | Check actual shielded deposit, network, scan progress, confirmations, and spendable balance                                                        |
| Payout queued/preparing                    | Check worker health and proving paths; proof generation and confirmed change can take time                                                         |
| Payout paused                              | Read the recorded state and recovery guide; do not reset or create a replacement transaction                                                       |
| Receipt status error                       | Keep the complete token fragment and use refresh; server acceptance alone is not payment confirmation                                              |
| Email rejection or zero accepted messages  | Check verified sender, provider permissions, organizer token, and displayed provider error                                                         |
| QR hidden in email                         | Enable images or open the included claim link; verify its public origin is reachable from the phone                                                |
| `crypto.randomUUID` unavailable            | The helper falls back to secure `getRandomValues`; use HTTPS and a supported browser; no insecure randomness fallback exists                       |
| Funds absent in recipient wallet           | Sync the correct testnet claim wallet from a suitable birthday and check the receipt transaction; do not confuse a local sweep report with funding |

## Limitations and next steps

- Integrate and verify an Ironwood-compatible browser wallet, including encrypted recovery, reload survival, incoming note detection, and locally signed sweeps. Current external-wallet entry does not complete the original automatic browser-wallet workflow.
- Replace shared organizer tokens with reviewed identity/session management. Keep development funding and experimental signing endpoints disabled; the GCP deployment does this by default.
- FROST uses a dealer-generated Ed25519 demonstration that returns signing packages and accepts shares on the server. It is not distributed key generation, secret-isolated cosigning, or threshold Zcash transaction authorization.
- Preserve the difference between a reservation, queued payment, broadcast, mined payment, policy confirmation, and self-reported sweep. Expiration releases unused reservations; it cannot reclaim already funded recipient notes.
- Verify deployment, backup restoration, audits, live reorg recovery, and sustained performance. Testnet success does not establish mainnet safety or complete privacy against correlation.
- Keep gift-card fulfillment out of advertised functionality until a real provider flow is implemented and verified.
- The current browser can store organizer credentials and bearer tickets in localStorage. A public deployment needs a reviewed retention/session design and careful XSS defenses; browser storage is not a secure secret vault.

## Documentation, provenance, and licensing

- [Product brief](z-split-project-info.md) — original problem statement and proposed features.
- [Backend specification](Z_SPLIT_BACKEND_ARCHITECTURE.md) — implementation goals and security requirements.
- [Backend guide](zsplit-backend/README.md) — backend entry point and operator links.
- [Shared contracts](zsplit-backend/docs/contracts.md) and [API contract](zsplit-backend/docs/api-contract.md).
- [Redemption architecture decision](zsplit-backend/docs/adr-001-redemption-model.md).
- [Privacy and threat model](zsplit-backend/docs/privacy-and-threat-model.md).
- [Browser wallet release requirements](zsplit-backend/docs/browser-wallet-compatibility.md).
- [Recovery and latest validation evidence](zsplit-backend/docs/payout-recovery.md).
- [Historical benchmark report](zsplit-backend/docs/benchmark-report.md).
- [Builder-pack alignment and provenance](zsplit-backend/docs/builder-pack-alignment.md).

The repository contains frontend work predating the event and backend work added during the event. Preserve that history. Hackathon eligibility and submission acceptance are organizer decisions; a README or backend export does not establish eligibility. No partnership, audit certification, public deployment, or completed browser-wallet demo is implied.

The Rust workspace declares `MIT OR Apache-2.0` in its package metadata. Standalone root license texts are not currently included; add the intended project license files before presenting the repository as a fully licensed public release. Font license notices are included in `static/fonts/`. Dependencies retain their respective licenses.
