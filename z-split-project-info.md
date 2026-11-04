# Z-Split: Shielded Room Payouts (Zero-Address Collection)

> Pay 50 people in a room without ever collecting a single wallet address.

---

## 1. One-Line Pitch

Z-Split lets an organizer pay hackathon winners, contributors or event participants in shielded ZEC through one-time claim links and QR codes, so no spreadsheet of identities-to-addresses ever exists.

---

## 2. The Problem

Paying a group of people in crypto today usually works like this:

1. The organizer asks everyone for a wallet address.
2. Addresses get pasted into a spreadsheet, form or chat next to names and emails.
3. The organizer sends payments one by one.

This creates several problems:

- **Identity-to-address linkage.** The spreadsheet permanently links real people to on-chain addresses. If it leaks, or the organizer is compromised, recipients are doxxed financially.
- **Manual overhead.** Collecting, verifying and copy-pasting 50 addresses is slow and error-prone. One typo means lost funds.
- **Privacy defeated by process.** Even if the payment is shielded, the off-chain paperwork undoes the privacy.
- **Non-crypto users are excluded.** Many recipients don't have a wallet and don't want to set one up just to get paid.

"Shielded payouts without collecting wallet addresses by hand" is listed on the Zcash hackathon builder wishlist as an unsolved, high-priority tool.

---

## 3. The Solution

The organizer funds a **coordinator pool** with shielded ZEC and generates **one-time claim links / QR codes**, one per recipient. Each claim is bound to an amount. Recipients redeem their claim themselves, in whichever way suits them:

- **Option A: Sweep to own wallet.** Open the link in a lightweight WebZjs browser interface and sweep funds into their own shielded wallet.
- **Option B: Scan with a wallet.** Scan a ZIP-321 payment URI QR directly with a compatible wallet.
- **Option C: Cash out.** Convert immediately into a Bitrefill gift card for recipients who don't want to hold crypto.

The organizer never learns a recipient's address. Recipients never have to share one.

---

## 4. How It Works

### 4.1 Flow

```
Organizer                 Coordinator Pool              Recipient
    |                            |                          |
    | 1. Deposit shielded ZEC    |                          |
    |--------------------------->|                          |
    | 2. Generate N claims       |                          |
    |    (ephemeral keys,        |                          |
    |     amounts)               |                          |
    |<---------------------------|                          |
    | 3. Distribute QR / links   |                          |
    |------------------------------------------------------>|
    |                            |  4. Open link / scan QR  |
    |                            |<-------------------------|
    |                            |  5. Sweep to own         |
    |                            |     shielded wallet      |
    |                            |     OR Bitrefill voucher |
    |                            |------------------------->|
    | 6. Dashboard: claimed /    |                          |
    |    unclaimed (no addresses)|                          |
```

### 4.2 Claim Mechanism (two possible designs)

**Design 1: Ephemeral spending keys (recommended for MVP)**

- For each recipient, the organizer's client generates a fresh ephemeral Zcash shielded keypair.
- The organizer sends the exact payout amount to that ephemeral address (shielded).
- The spending key is encoded into the claim link (in the URL fragment, so it never reaches a server) and rendered as a QR code.
- The recipient opens the link, WebZjs derives the ephemeral wallet from the key, syncs, and sweeps the funds to the recipient's own shielded address.
- Whoever holds the link holds the funds until it is swept, like a paper bearer note.

**Design 2: ZIP-321 payment URIs**

- The coordinator generates ZIP-321 payment request URIs per recipient.
- The recipient scans and pays out to their own address via a compatible wallet.
- Simpler, but it needs more coordination for the recipient to provide a destination.

### 4.3 Cash-Out Path (Bitrefill)

- After claiming, the recipient can choose "Spend as gift card."
- The app creates a Bitrefill order for the claimed amount and pays from the swept funds.
- The recipient receives a gift card code with no wallet and no crypto exposure.

---

## 5. Architecture

| Component | Responsibility | Tech (suggested) |
|---|---|---|
| Organizer dashboard | Fund pool, define recipients/amounts, generate claims, track status | Web app (React/Next.js) |
| Claim generator | Create ephemeral keys, fund them, build links and QR codes | WebZjs / Zcash SDK in the browser |
| Claim page | Recipient UI: sweep to wallet or cash out | WebZjs, static page |
| Status service (optional) | Track claimed vs unclaimed without storing addresses | Lightweight backend or on-chain checks |
| Off-ramp module | Convert claimed funds into a Bitrefill gift card | Bitrefill API |

**Design principles**

- **Client-side key handling.** Keys are generated in the browser and carried in URL fragments. The backend never sees spending keys.
- **No address storage.** The system records claim status, never recipient addresses.
- **Stateless where possible.** Fewer servers means fewer things to trust or breach.

---

## 6. Key Features

**MVP (hackathon scope)**
- [ ] Organizer funds a pool and creates N one-time claims with amounts
- [ ] Per-recipient QR code and link generation (ephemeral keys)
- [ ] Printable or shareable QR sheet for a live room
- [ ] Claim page that sweeps funds into the recipient's shielded wallet
- [ ] Organizer dashboard showing claimed / unclaimed counts

**Stretch**
- [ ] Bitrefill gift-card cash-out option
- [ ] Claim expiry with refund of unclaimed funds to the organizer
- [ ] CSV-free bulk setup (amounts or tiers only, no identities)
- [ ] Optional memo field for payout label (for example "1st place")
- [ ] Multi-organizer or multisig pool funding

---

## 7. Why It Matters

- **Real use case today:** hackathons, bounties, grants, DAO contributor pay, community rewards, event giveaways, journalism and activist funds where recipients need to stay private.
- **Removes the biggest privacy leak in group payouts:** the address spreadsheet.
- **Lowers the barrier for non-crypto users** through the gift-card option.
- **Directly on the Zcash wishlist,** so the problem is already validated by the ecosystem.

---

## 8. Hackathon Track Fit

| Track | Fit | Pitch angle |
|---|---|---|
| **Privacy (Zcash)** | Strong | Shielded payouts with zero address collection; ephemeral keys; fragment-only secrets |
| **Agentic Commerce (Bitrefill)** | Moderate | Instant ZEC to gift card cash-out for recipients who don't want crypto (strengthen this with an agent-driven payout option) |
| **Zcash Ecosystem Prize (30 days)** | Strong | Milestone: production-ready claim flow plus expiry/refund, packaged as a reusable library |
| **Mentorship** | Yes | Feedback on key handling and WebZjs sync performance |

---

## 9. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| **Bearer-link theft.** Anyone with the link can claim. | Short expiry, one-time use, delivery by QR in person, optional PIN or passphrase layer |
| **WebZjs sync time** on the claim page is slow for new ephemeral wallets | Use birthday heights set to the funding block; pre-fund close to the demo |
| **Coordinator trust.** The organizer holds funds before claim. | Be upfront: this is no worse than any payroll; add expiry and refund; consider multisig as a stretch |
| **Fees and dust.** Many small notes mean overhead. | Batch funding transactions; set minimum payout; show fee estimate |
| **Network-level privacy.** IP or timing correlation at claim time. | Document it honestly; recommend claiming over Tor or a VPN |
| **Bitrefill availability or ZEC support** | Verify catalog and payment support early; keep cash-out as a stretch feature |

---

## 10. Demo Plan (live in the room)

1. Organizer funds a pool with testnet or small mainnet ZEC.
2. Generate 5 to 10 claim QR codes live on screen.
3. Invite judges or audience members to scan a code with their phone.
4. Watch the dashboard flip from "unclaimed" to "claimed" with no address ever shown.
5. Show a second recipient choosing the gift-card cash-out path.
6. Close with: *"We just paid a room of people, and nobody collected a single address."*

---

## 11. 30-Day Milestone (Ecosystem Prize)

**Proposed single milestone:** ship a reusable open-source "Z-Split" library and hosted claim page with:

- Claim generation and sweep flow audited for key handling
- Expiry and automatic refund of unclaimed funds
- Documentation so any Zcash community can run their own payout room

---

## 12. Team and Roles (fill in)

| Role | Name | Responsibility |
|---|---|---|
| Lead / Product | | Pitch, scope, demo |
| Zcash / WebZjs dev | | Key generation, sweep logic |
| Frontend | | Organizer dashboard, claim page |
| Integrations | | Bitrefill cash-out, QR/printing |

---

## 13. Tech Stack (suggested)

- **Zcash:** WebZjs, Zcash shielded (Orchard/Sapling) transactions, ZIP-321 URIs
- **Frontend:** React or Next.js, QR code library
- **Backend (optional):** minimal service for claim-status tracking
- **Off-ramp:** Bitrefill API
- **Hosting:** static hosting for the claim page

---

## 14. Links and References

- Zcash hackathon builder wishlist (item: shielded payouts without manual address collection)
- ZIP-321: Payment Request URIs
- WebZjs documentation
- Bitrefill API documentation

---

## 15. Tagline Options

- *"Pay the room. Collect nothing."*
- *"Zero addresses. Zero spreadsheets. Full privacy."*
- *"Scan. Claim. Done."*
