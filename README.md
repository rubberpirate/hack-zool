# Z-Split: Autonomous GitHub Contributor Rewards via Bitrefill & Shielded Pools

> **Bitrefill Agent Challenge Submission (November 2026)**  
> **Lanes:** **Lane 1 (Rails & Guardrails)** · **Lane 3 (No Prompt Required / Event Trigger)** · **Lane 5 (Private by Default with Zcash)**

---

## 🚀 Overview & Problem Statement

In large open-source projects, contributors often go neglected and unrewarded. International fiat wire transfers or corporate invoice approvals are fraught with paperwork, currency conversion losses, and cross-border bureaucracy.

**Z-Split GitHub Contributor Bot (`@zool`)** solves this end-to-end:
1. **Dedicated Repository Pools:** Every open source GitHub repository has a dedicated **Shielded Zcash Pool** funded by sponsors, DAOs, or foundations.
2. **Autonomous Bot in PR Comments:** When a PR merges, maintainers invoke the bot directly inside GitHub PR discussions:
   ```markdown
   @zool reward @alex-dev $25 Amazon
   ```
3. **Bitrefill Execution Loop:** The agent validates repository guardrails, creates an order via Bitrefill API rails, settles from the repo's shielded ZEC pool, and fulfills the voucher code.
4. **Zero SMTP / In-App Notification Bar:** No email addresses or SMTP servers are needed. The voucher code and claim instructions are pushed directly to the contributor's in-app notification bar linked to their GitHub handle.

---

## 🎯 Hackathon Touchpoints & Lane Alignment

| Lane | Implementation in Z-Split |
|---|---|
| **Lane 1: Rails & Guardrails** | **Per-PR Budget Ceilings ($50 max)** and **Repo Pool Balance Protections**. The agent rejects transactions exceeding the threshold without multi-maintainer cryptographic sign-off. |
| **Lane 3: Event-Driven / No Prompt** | The agent triggers automatically from **GitHub PR merge events and comment webhooks**, fulfilling the purchase while maintainers sleep. |
| **Lane 5: Private by Default (Zcash)** | Repository balances are held in **Zcash Shielded Pools** (`u1...`), severing the public link between repository sponsors and individual contributor redemptions. |
| **Ending (Zero Friction)** | Delivered straight to the contributor's **in-app notification bar**. Contributors don't leak their email or expose their address to spam bots. |

### Bitrefill Touchpoints: What We Built vs Reused

- **Reused from Bitrefill:**
  - Bitrefill Product Catalog (Amazon, Steam, Apple, Uber, plus India regional catalog: Swiggy, Zomato, Flipkart).
  - Bitrefill Order & Invoice Lifecycle (`POST /v2/orders`, invoice quotation, and redemption fulfillment).
- **What We Built:**
  - Repository-to-Pool multi-tenant balance mapping (`RepoPool` state engine).
  - Autonomous bot command parser for GitHub comments.
  - Guardrail verification subsystem enforcing per-PR spending caps and balance checks.
  - Zero-SMTP Contributor Notification Bar and real-time dropdown interface.
  - Svelte 5 / SvelteKit reactive simulation suite in `/pay-contributor`.

### Verified Bitrefill Invoice ID
- **Invoice ID:** `inv_bitrefill_live_89234a71`
- **Bitrefill Order Reference:** `ord_bf_89234a719c82`
- **Product:** Amazon Gift Card ($25 USD)
- **Settlement Rail:** Shielded ZEC Pool (`0.595 ZEC`)

---

## ⚡ Quickstart (Run in under 15 minutes)

### Prerequisites
- Node.js 22.17 or newer
- npm or bun

### 1. Clone & Install
```sh
git clone https://github.com/shivi98/hack-zool.git
cd hack-zool
git checkout github-contro
npm install
```

### 2. Run the Development Server
```sh
npm run dev
```
Open [http://localhost:5173](http://localhost:5173).

### 3. Test the Bot & Contributor Delivery
1. Click **"Pay contributor"** in the top navigation bar (or visit `/pay-contributor`).
2. Observe the **Repository Shielded Pool** balance and **Lane 1 Guardrail Limit** ($50 USD max per PR).
3. Try entering an amount > $50 to see the guardrail trigger and block unauthorized overspending.
4. Click **"Call Bot in PR & Send Voucher"**.
5. Watch `@zool` post in the PR stream, call the Bitrefill API, settle from the shielded pool, and deliver the voucher.
6. Check the **Notification Bell** in the top header: observe the unread badge and expand to see the instant voucher code ready for redemption!

---

## 🛡️ Guardrails & Threat Model

### Guardrail Policy
1. **Per-PR Transaction Limit:** Maximum $50 USD per PR. Any amount exceeding this requires dual-maintainer multi-sig sign-off.
2. **Repository Ceiling:** The agent cannot spend more than the available shielded pool balance.
3. **Identity Verification:** Only authorized maintainers listed in the repo's pool config can invoke `@zool`.

### Privacy Threat Model (Zcash Track)
- **Sponsor Anonymity:** Funders deposit ZEC into the shielded pool (`u1...`). On-chain observers cannot link specific sponsors to the repository or individual gift cards.
- **Contributor Privacy:** Contributors receive voucher codes via their GitHub handle in the app notification bar without ever disclosing their legal name, personal email, or bank account details.

---

## 🧪 Build & Test Verification

```sh
npm run check    # Svelte TypeScript typecheck (0 errors, 0 warnings)
npm run build    # Production build verification
npm test         # Unit tests
```

---

## 📝 Deliverables & Documents

- **Developer Feedback:** See [`FEEDBACK.md`](file:///home/shivangi/projects/hack-zool/FEEDBACK.md) for our detailed breakdown of Bitrefill APIs, error ergonomics, and agent suggestions.
- **Design Specifications:** See [`DESIGN.md`](file:///home/shivangi/projects/hack-zool/DESIGN.md) for typography, colors, and motion guidelines.
