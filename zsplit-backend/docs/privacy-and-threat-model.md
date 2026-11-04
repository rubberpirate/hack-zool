# Privacy and recovery contract

Z-Split is custodial until its coordinator funds a browser-created ephemeral Orchard wallet. It protects the recipient's personal sweep destination by keeping that operation on the recipient's device. It does not make a recipient anonymous to every observer or make a dishonest coordinator trustworthy.

## Secrets and records

| Data | Where it exists | Retention and exposure |
| --- | --- | --- |
| Claim ticket | Issuance response, organizer's export, recipient browser | 32 random bytes encoded as 43 URL-safe characters; bearer possession authorizes redemption. Return once. Never write raw tickets to a database, log, URL query, or server path. |
| Ticket digest | PostgreSQL | Domain-separated HMAC-SHA256. The independent HMAC secret belongs in deployment secrets. Losing or rotating it without a migration invalidates outstanding tickets. |
| Recipient ephemeral spending key | Recipient device and recipient-controlled backup | The backend has no API field for it. It must never be sent to the coordinator, including in telemetry or support requests. |
| Ephemeral receiving address | Redemption request, wallet worker, encrypted pending queue, native sender history | ChaCha20-Poly1305 with random nonce and claim ID associated data protects the application queue. Clear queued receiver ciphertext once durable transaction state permits. The native wallet may retain output metadata and must have a protected volume. |
| Personal sweep address | Recipient's local wallet operation | Never requested by the API, dashboard, or sweep-report endpoint. |
| Coordinator seed | Protected operator secret file/volume | Required to recover coordinator spending authority. Native wallet viewing/transaction data is sensitive even without the seed. |
| Signed transaction | Native wallet and durable payout recovery record | Rebroadcasting identical bytes is safe against duplicate payout intent. Restrict database/backup access because transaction metadata can correlate activity. |

The claim ticket is a capability, not a Zcash key. QR data uses `/claim/{claim_id}#t={ticket}`. Browsers must read the fragment locally and send the ticket only in a redacted `Authorization: Bearer` header. A fragment is not sent in the HTTP request, but scripts and extensions can read it. Use a strict Content-Security-Policy, `Referrer-Policy: no-referrer`, HTTPS, and no third-party analytics on claim pages. Do not include receiver values, bearer headers, seeds, viewing keys, memos, or raw request bodies in tracing spans.

## Authorization and abuse

Tickets carry 256 bits of OS entropy. The library accepts only canonical unpadded base64url representations and compares fixed-length digests in constant time. Encryption keys and ticket HMAC keys must be independently generated 32-byte secrets. A database leak without those secrets cannot decrypt pending receivers or derive raw tickets.

A ticket authorizes its claim only. Claim lookup, status, redemption, and sweep-report routes must verify the digest without revealing whether an unauthenticated claim exists. Apply bounded rate limits before database work and cap request sizes. Bound any per-IP or per-token limiter state; do not retain raw bearer tokens as limiter keys. Proxy headers are trustworthy only behind an explicitly trusted reverse proxy. Global rate limiting may affect unrelated users behind shared networks; it is abuse mitigation, not proof of recipient identity.

The organizer can copy every ticket before distributing it. Anyone who steals a ticket can win its first redemption. One-time redemption prevents a second payout; it cannot identify the rightful holder or undo an on-chain theft. Optional PINs, identity checks, and gift-card conversion are outside this MVP.

## Financial authority and failure handling

Only native wallet synchronization establishes confirmed funding and spendability. A successful HTTP redemption queues one durable intent; it does not establish a payment. A successful broadcast is also not a confirmation. The dashboard must distinguish queued, preparing, broadcast, unconfirmed, confirmed, reorged, and manual-review states.

The claim row transition and unique payout job are one PostgreSQL transaction. A matching idempotent retry returns its original operation; a competing operation cannot create a second job. Queue workers serialize wallet spending, persist the signed transaction before broadcasting, and rebroadcast only the same bytes on uncertain outcomes. A crash while native transaction construction has started requires native intent reconciliation or manual review. It must never automatically restart a new payout transaction. Recovery favors temporarily locked funds over duplicate payments.

A chain reorganization may invalidate a confirmation or spendability snapshot. Downgrade status and reconcile using the native wallet; retain liabilities until the outcome is known. A formerly confirmed payment is not grounds to make a replacement payment automatically.

Expiry releases only an unredeemed reservation. It neither revokes an ephemeral spending key nor claws back an already funded note. Queued and ambiguous jobs remain liabilities after the displayed ticket expiry time. Unused funds remain with the coordinator until an authenticated withdrawal of unreserved, spendable funds succeeds, including its fee budget.

A sweep report is a client assertion in a separate field. It must never replace `funded_confirmed`, increment a verified-sweep counter, prove recipient possession, or authorize accounting releases. Verifying a personal sweep would require additional viewing data or another privacy-sensitive mechanism that this backend does not request.

## Device loss and external observers

Before redemption, the client must create an ephemeral wallet locally, persist it across reloads, and provide a recovery/export path. The backend requires a persistence attestation but cannot prove that the browser actually stored the key. Clearing browser storage, losing the backup, private browsing, malware, or an extension can still lose funds permanently. A funded ticket cannot restore the recipient's missing spending key.

The coordinator, lightwalletd operator, gRPC-Web proxy, hosting provider, and network observers can observe connection metadata or correlate timing. TLS protects transit; it does not hide IP addresses from the service being contacted. Optional Tor/VPN use and discreet ticket distribution can reduce some exposure but provide no blanket anonymity guarantee.

Protect volumes and backups with filesystem permissions and appropriate disk encryption. Restore PostgreSQL, native wallet data, coordinator seed, ticket HMAC secret, and receiver encryption secret consistently. Test recovery with test funds before real-value use. Do not call this implementation audited or mainnet-safe on the basis of compilation or unit tests.

See [browser wallet compatibility](browser-wallet-compatibility.md) for the separate recipient implementation gate. The semantics of a payment-request URI are defined by [ZIP 321](https://zips.z.cash/zip-0321); it does not define a bearer-ticket redemption protocol.
