# Recipient wallet compatibility gate

The Rust backend does not create recipient spending keys or sweep funds. A real browser receive-and-sweep test is required before calling the complete product operational. The repository's existing frontend mock flow is not evidence of this gate passing.

## Upstream evidence checked 2026-11-04

The [WebZjs README](https://github.com/ChainSafe/WebZjs/blob/a50df944c32243cb8da9f86e7d52cb65ac926439/README.md) documents `@chainsafe/webzjs-wallet`, Wasm initialization, a worker thread pool, account creation, and synchronization. It requires a browser-compatible gRPC-Web proxy rather than a direct native lightwalletd endpoint, and states that its libraries have not received a security review or audit.

The [wallet crate manifest](https://github.com/ChainSafe/WebZjs/blob/a50df944c32243cb8da9f86e7d52cb65ac926439/crates/webzjs-wallet/Cargo.toml) enables Orchard in its Zcash dependencies and provides Wasm/parallel build features. These are source-level capabilities, not a Z-Split browser compatibility certification. The [wallet implementation](https://github.com/ChainSafe/WebZjs/blob/a50df944c32243cb8da9f86e7d52cb65ac926439/crates/webzjs-wallet/src/wallet.rs) exposes account import, synchronization, proposal construction, proof/signing, and database export code. Its dependency versions differ from this backend's lockfile: reuse protocol encodings and test interoperable transactions, not Rust data structures across those boundaries.

Pin the chosen browser package/build and record its version or commit in the deployment. The upstream commit above was resolved from its repository HEAD during implementation. A reviewed source tree does not establish that a particular npm artifact matches it. Upstream [documentation](https://chainsafe.github.io/WebZjs/) is a starting point for the client integration.

## Client requirements

1. Initialize the selected Wasm build once. For threaded builds, serve cross-origin-isolated pages with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, ensure all worker/Wasm resources meet those headers, and check `crossOriginIsolated`/thread availability on the target browser. A supported single-thread build may be used only after measuring its responsiveness and proof time.
2. Read the QR fragment locally. Keep the bearer ticket out of URL queries, Referer, logs, analytics, and exception messages. Do not transform the ticket into a spending key or a `zcash:` payment URI.
3. Generate entropy and an ephemeral wallet entirely on the recipient device with the wallet's supported key API. Require an explicit network match and an Orchard-capable unified address. Never POST seed material or spending keys to the backend.
4. Persist wallet state and provide a recipient-controlled recovery export. Verify restoration across reload before attesting `wallet_persisted: true`; a JavaScript boolean alone cannot guarantee durable storage. Block redemption if storage/export fails or the browser mode cannot provide recovery.
5. Send only the fresh ephemeral receiver, client network, stable idempotency UUID, and persistence attestation in the redeem body. Send the ticket through the Authorization header. Preserve the same receiver and idempotency UUID for retries; the original wallet must remain recoverable after request timeouts.
6. Synchronize through a compatible gRPC-Web proxy for the chosen network and scan from a birthday at or before payout construction. Store network and birthday with the client backup. Detect the actual incoming shielded note and required confirmation depth independently of the API's accepted response.
7. Ask for the personal sweep address only in the recipient wallet UI. Build, prove, sign, and broadcast that sweep locally through the recipient wallet's network connection. Never send its destination, viewing key, spending key, or memo to Z-Split. Account for the native wallet's ZIP-317 fee requirements and insufficient spendable balance.
8. A sweep-report request is optional and sends only the fact the user reports completion. Render it as unverified; backend funding confirmation remains a separate fact.

A unified address may contain multiple receiver types. Backend validation requires a valid Orchard receiver and exact mainnet/testnet/regtest encoding. The sender must select Orchard rather than fall back to a transparent receiver. A generic wallet's ZIP-321 support does not imply support for this application protocol: [ZIP 321](https://zips.z.cash/zip-0321) asks a wallet to construct payments and does not transfer ticket authority or create an ephemeral wallet.

## Required compatibility evidence

Record the browser/OS, upstream package/build, network, gRPC-Web endpoint, Wasm threading mode, confirmation policy, and client storage/restore procedure. With test funds, demonstrate: a new wallet survives reload; a 202 redemption timeout can be retried without a second funding job; an incoming Orchard note is detected; a local sweep signs/broadcasts and is detected by a separate destination wallet; coordinator requests and logs contain no recipient key or personal sweep address. Repeat with private/incognito or storage-disabled mode to confirm funding is blocked, and exercise a reload while the payout is pending.

**Current result:** the claims library validates native Orchard receiving addresses and enforces its persistence policy helper, but no browser wallet receive-and-sweep session has been executed as part of this agent's work. This document describes a release gate and does not certify the existing frontend or upstream package.
