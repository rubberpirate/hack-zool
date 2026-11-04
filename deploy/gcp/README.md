# GCP deployment — Zool testnet

This deployment uses one Compute Engine VM in Mumbai. A single process owns the native wallet; PostgreSQL, web, API, and Caddy run in Docker Compose. The API and database are reachable only inside the Compose network. Caddy publishes ports 80/443 and automatically manages the site's HTTPS certificate after DNS is correct.

## Provisioned environment

- Project: `project-547f14f6-1ba3-45a5-ad5`
- Instance: `zool-server`; zone: `asia-south1-a`
- Machine: `e2-standard-4` (4 vCPU, 16 GB RAM)
- Boot/storage: 100 GB balanced persistent disk, automatic deletion disabled
- Reserved external address: `34.47.183.93` (`zool-public-ip`)
- Domain: `zool.rubberpirate.in`
- Web rule: `zool-web`, TCP 80/443, matching VM tag only
- VM service account/scopes: none

The instance, storage, external address, and traffic incur GCP charges. This is a single-VM demo deployment, without high availability, automated backups, or a production service-level guarantee. Stopping the VM leaves disk and address costs. The existing `proofbid-server` instance is unrelated and was not changed.

The VM has a **fresh** testnet seed and application database. Local rooms, tickets, wallet seed and native journal were not migrated. Do not use old localhost tickets as evidence of a cloud payment. Reusing a sender wallet on two live machines is unsafe.

## DNS and access

Create an A record `zool.rubberpirate.in → 34.47.183.93`. For Cloudflare, initially use DNS-only mode so ACME validation reaches Caddy. Remove conflicting AAAA records or point them correctly. Access:

- App: `https://zool.rubberpirate.in`
- Organizer sign-in: `https://zool.rubberpirate.in/organizer`
- Liveness/readiness: `/healthz`, `/readyz`
- API schema: `/openapi.json`

The organizer token is generated independently from development secrets. It is not in the client bundle. On the operator's local machine it is stored in `~/.local/share/zool/gcp-organizer-token.txt`, mode 0600. The complete cloud configuration is in `~/.local/share/zool/gcp-production.env`, also mode 0600. These files are outside the repository. Do not share them in issues or commit them.

The sign-in page verifies the token server-side and stores it in the organizer browser for API calls. This remains shared-token authentication, not multi-user identity, revocable sessions, or hardware-backed custody. Use a trusted browser. Recipient links never require the organizer token.

## Files and bootstrap

On the VM:

```text
/opt/zool/app/                    source and Docker build contexts
/opt/zool/config/production.env   root-owned environment, mode 0600
/opt/zool/state/postgres/         PostgreSQL data
/opt/zool/state/zcash/            seed, native wallet/journal, public proving files
/opt/zool/state/caddy/            certificate/account storage
```

`bootstrap-host.sh` installs Docker from its signed Debian repository and creates the base directories. Run it only on the intended fresh Debian 12 VM as root. Upload a source bundle that excludes `.git`, `.env*`, local wallet data, seeds, caches, build output, and dependency directories. Upload the generated production environment separately over SSH and install it as `/opt/zool/config/production.env` with mode 0600; do not put secrets in instance metadata or build arguments.

Required environment keys are `DOMAIN`, `POSTGRES_PASSWORD`, `ORGANIZER_ID`, `ORGANIZER_API_TOKEN`, `CLAIM_TOKEN_KEY`, and `RECEIVER_ENCRYPTION_KEY`. Use independent cryptographic random secrets; the two application keys must be 64 hex characters each. Optional email keys are `RESEND_API_KEY` and `RESEND_FROM_EMAIL`. The supplied Compose file sets testnet, three confirmations, disabled interactive docs, and disabled development endpoints.

After provisioning directories, source, and configuration:

```sh
sudo bash /opt/zool/app/deploy/gcp/start.sh
```

The script validates Compose, builds both applications, verifies public prover parameters, creates a protected seed only if no wallet exists, and starts the stack. It refuses to replace a missing seed belonging to an existing wallet. Back up the resulting seed and wallet before funding.

## Operations

From `/opt/zool/app` on the VM:

```sh
sudo docker compose --env-file /opt/zool/config/production.env -f deploy/gcp/compose.yaml ps
sudo docker compose --env-file /opt/zool/config/production.env -f deploy/gcp/compose.yaml logs --tail=60 api
sudo docker compose --env-file /opt/zool/config/production.env -f deploy/gcp/compose.yaml logs --tail=60 proxy
```

Do not print `docker compose config` or container environment output: it contains resolved secrets. Use `config --quiet` for validation. Access logs are not enabled in Caddy, so bearer fragments and email payloads are not intentionally recorded there. API logs avoid authorization headers and receivers.

The API has a five-minute container stop grace period. Do not hard-kill it during proving or replace the native intent journal. See [payout recovery](../../zsplit-backend/docs/payout-recovery.md) for ambiguous outcomes. Docker's restart policy resumes existing containers after a reboot; persistent data is stored outside the source tree.

Before updating, back up the state/configuration and record a source revision. Rebuild changed images and run Compose `up -d`. API startup applies migrations. Rollback must respect schema compatibility; restoring only an old application image does not roll back migrations or native wallet state. Do not use `down -v`, remove state directories, or replace the coordinator seed to fix startup.

Backups must include PostgreSQL, the original seed, native wallet and intent journal, and configuration keys. Stop wallet work gracefully for a coordinated filesystem backup and use a proper PostgreSQL backup/restore procedure. No automated backup or restore drill is included in this demo deployment.

## Public surface and verification

The Rust API disables development routes unless `ENABLE_DEVELOPMENT_ENDPOINTS=true`. Production Compose sets it false, and Caddy additionally blocks `dev-confirm`, FROST, and interactive docs. Development shortcuts/FROST controls are omitted from the production UI. The configured organizer secret is required for organizer operations and email; no public development fallback token is accepted by this deployment.

Verify the home page, organizer sign-in, real room creation and deposit address, unauthenticated organizer rejection, blocked dev/FROST endpoints, email configuration without sending unsolicited mail, and absence of secrets in browser assets. Run public checks from the repository root with `node deploy/gcp/verify.mjs`. Set `ZOOL_ORGANIZER_TOKEN` in the process environment to include authenticated checks; adding `--create-room` deliberately creates one real, unfunded smoke-test room. The script never simulates a deposit or broadcasts a transaction.

The cloud wallet starts unfunded: a successful health check or room creation is not a blockchain payment. A new genuine payout requires an actual shielded testnet deposit, claim redemption, native broadcast, and confirmations. Historical local confirmed transaction IDs remain separate evidence.

## Recorded deployment verification

See [verification-results.json](verification-results.json). Public HTTPS, readiness, API metadata/schema, authentication rejection/acceptance, and blocked development routes passed 14 checks. A separate mobile Chromium session signed in, created a real room through the API, and rendered its native testnet deposit address without development controls. Rust checks, 14 frontend unit tests and 10 targeted browser regressions passed; the production npm audit reported zero advisories. The five historical landing/demo browser failures were not part of this targeted run. Two unfunded verification rooms were created; no tickets were funded, no emails sent, and no transaction broadcast on the cloud wallet.
