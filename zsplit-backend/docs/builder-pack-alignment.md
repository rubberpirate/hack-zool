# Free Society Hack builder-pack alignment

Reviewed 2026-11-04: [Builder pack](https://app.notion.com/p/Free-Society-Hack-Builder-Pack-3ff3e52aeb15807eace6f16f84a74c14), [Privacy brief](https://www.notion.so/4073e52aeb1580e9b24fe57aeddb8b3e), [Zcash resources](https://www.notion.so/3ff3e52aeb1580a583c8e82b95a6f94e). These are living pages; deployment must recheck the active network and SDK. Sources were read through Notion's public page endpoint after the connected Notion fetch tool failed.

The selected theme is private payment rails. Pitch: organizers reserve confirmed shielded testnet funds and distribute one-time tickets; recipients redeem into a locally controlled ephemeral wallet while keeping their spending key and personal sweep destination off the server.

## Changes prompted by the pack

- Use NU7 testnet and the working `https://testnet.zec.rocks:443` upstream. Refuse mainnet and unsupported network/branch configurations.
- Distinguish the Orchard SDK feature and unified-address receiver from the active value pool. Official backend 0.25, keys 0.17 and protocol 0.11 support Ironwood. Post-NU6.3 new recipient outputs must target Ironwood; existing Orchard notes can require protocol-aware migration. Count both pools accurately and let the SDK build transactions and ZIP-317 fees.
- Do not advertise WebZjs as compatible with current testnet without a demonstrated Ironwood-capable release. Native recipient CLI testing can establish payment functionality; it does not establish the requested browser wallet workflow.
- Preserve seen, mined and policy-confirmed states. A successful broadcast and a client sweep report are insufficient confirmation evidence.
- Keep Bitrefill outside the backend MVP. Entering Agentic Commerce requires a genuine autonomous purchase and invoice evidence; the current gift-card mock satisfies neither requirement.
- Document the four privacy questions: what is private, what is exposed and to whom, which parties are trusted, and what must be fixed next. See privacy-and-threat-model.md.

## Submission evidence and provenance

This repository contains a pre-event Svelte demonstration with commits predating the event. It must not be represented as event-created work. The Rust backend was added on 2026-11-04 in separate commits. The pack prohibits building an entry on a project started before the event and asks for a new repository: whether this backend is eligible requires organizer review. Do not rewrite or fabricate commit history to hide prior work. A clean backend-only export can be made with scripts/export-backend.sh, retaining a provenance note; export alone does not establish eligibility.

A backend entry can present a reproducible README/CLI rather than a public app URL, but still needs a genuine shielded testnet transaction ID. A deployed user-facing web app needs a public testnet URL. A video must show the real flow within three minutes. This repository does not claim any transaction, recording, deployment, participant check-in, partner conversation or user validation that has not actually occurred. Prepare a Stadium draft early; human submission and eligibility decisions remain with the team.

## Thirty-day next step

Proposed scope: demonstrate an Ironwood-capable browser recipient wallet with recovery, then run a reviewed 50-ticket pilot and publish failure-recovery evidence. Treat this as a proposed milestone, not an agreed partnership. The builder pack's ecosystem follow-through asks for public delivery and a partner confirmation by 2026-12-04. No partner is currently asserted.

## Actual friction notes

SQLx 0.8 conflicted with the native wallet's SQLite linkage; resolving a complete stable SDK family plus SQLx 0.9 fixed it. Swagger's previous build dependency had four ZIP security advisories; upgrading removed them. Tonic required explicit WebPKI TLS configuration and a crypto provider, discovered by a live gRPC probe. Older Orchard-only wallet instructions miss the current pool transition. Browser compatibility and live chain evidence remain independent gates from passing Rust and SQL tests.
