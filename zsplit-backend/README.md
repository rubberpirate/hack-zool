# Z-Split Rust backend

This workspace provides the native testnet payout backend for **Zool / Z-Split**: Axum APIs, PostgreSQL reservations and outbox jobs, official Zcash wallet persistence, and serialized transaction proving/broadcast/reconciliation.

The complete, current project guide is the [repository README](../README.md). It covers the technology stack, design decisions, setup, API routes, configuration, privacy, test evidence, and deployment limits. It supersedes earlier descriptions of this prototype as non-custodial, browser-wallet complete, or protected by enforced FROST treasury signing.

## Development startup

From this directory, after configuring `.env` from `.env.example` with fresh secrets and actual database credentials:

```sh
docker compose up -d postgres
# First installation only; default seed path is ./data/coordinator.seed.
cargo run --locked --package zsplit-zcash --example wallet -- seed
python3 scripts/download-proving-params.py
cargo run --locked --package zsplit-api
```

Do not recreate an existing seed. Set both proving parameter paths in `.env`. The seed example uses exported variables/default paths; the API loads the backend `.env` automatically. Default API listener: `127.0.0.1:8080`. SQLx migrations run on startup. The API and wallet worker share one process and one exclusive native wallet directory.

- Liveness: `/healthz`; dependency readiness: `/readyz`.
- Generated schema: `/openapi.json`; interactive Swagger: `/docs` when enabled.
- [Checked-in OpenAPI](docs/openapi.json), [contracts](docs/contracts.md), and [API contract](docs/api-contract.md).
- [Privacy and threat model](docs/privacy-and-threat-model.md).
- [Browser wallet compatibility requirements](docs/browser-wallet-compatibility.md).
- [Payout recovery and latest verification evidence](docs/payout-recovery.md).
- [Architecture decision](docs/adr-001-redemption-model.md).
- [Historical benchmarks](docs/benchmark-report.md).
- [Builder-pack alignment and provenance](docs/builder-pack-alignment.md).

## Quality checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
# Set DATABASE_URL to a disposable database before this command:
cargo test --locked --package zsplit-integration --test postgres -- --ignored
cargo run --locked --release --package zsplit-integration --bin zsplit-bench
```

The PostgreSQL fixtures require schema creation/deletion privileges. Setting `DATABASE_URL` also enables database benchmark fixtures. Unit tests, synthetic engine fixtures, and the `workflow_demo` example do not establish live chain success. Real confirmation evidence is recorded separately in the recovery report.

## Current boundaries

Testnet only; coordinator custody exists until payout. The web UI uses an external recipient claim wallet rather than an embedded browser wallet. Sweep reports are unverified. Experimental FROST routes are not threshold Zcash signing, and the development `dev-confirm` endpoint writes synthetic funding state. These routes require `ENABLE_DEVELOPMENT_ENDPOINTS=true`, which defaults to false; the GCP proxy blocks them as well. See the [GCP deployment guide](../deploy/gcp/README.md). Compose contains development credentials and requires explicit provisioning of seed/proving files; it is not a one-command production wallet installation.
