# Bitrefill Integration & Developer Experience Feedback

**Project:** Z-Split GitHub Contributor Rewards Bot  
**Submission:** Bitrefill Agent Hackathon (November 2026)  
**Author / Team:** Z-Split Team (`github-contro` branch)  
**Contact:** via GitHub repository  

---

## 1. Executive Summary
We built an autonomous GitHub Contributor Reward bot (`@zool`) that allows open-source maintainers to reward pull request authors with real gift cards and eSIMs directly from GitHub PR comments. The orders are settled through a repository-dedicated shielded Zcash (ZEC) pool and delivered via an in-app notification bar (zero SMTP, zero spam folder bounce).

During this hackathon, we interacted directly with Bitrefill's catalog structure, invoice creation workflow, and agent payment loop. Below is our honest, detailed feedback on what worked smoothly, what broke or confused us, and where the developer experience can be significantly improved.

---

## 2. What Worked Brilliantly

1. **Global & India Regional Catalog Variety:**
   - The catalog availability for both global gift cards (Amazon, Steam, Apple, Uber) and localized Indian services (Swiggy, Zomato, Flipkart) allowed our open-source pool to cater directly to global open-source developers, particularly in high-volume contributor hubs like India.
2. **Predictable Invoice & Order Lifecycle:**
   - The invoice state progression (`created` → `paid` → `delivered` → `redeemed`) is intuitive and maps directly to real-time agent event loops.
3. **Decoupled Delivery (Code + Redemption Link):**
   - Returning both the plain alphanumeric voucher code and redemption instructions makes it trivial to deliver vouchers via non-traditional channels (like GitHub comment notifications, Telegram bots, or our in-app notification center) without being trapped inside SMTP email delivery.

---

## 3. What Confused Us, Broke, or Slowed Us Down

### Issue 1: Agent Settlement via Shielded Cryptocurrencies (x402 / ZEC)
- **The Problem:** When an AI agent or automated bot pays an invoice on behalf of a pool (using shielded ZEC or private rails), it cannot supply a public transaction hash immediately or wait for 10 confirmations without timing out synchronous webhooks.
- **Friction:** Bitrefill's standard invoice checkout expects either an interactive user browser or a memo-based payment. Automated agent-to-agent settlement requires an instant zero-conf quote lock or a pre-funded merchant balance credit API that an agent can spend from programmatically.
- **Recommendation:** Provide a dedicated "Agent Pool Balance API" where a sponsor or DAO pre-funds an agent account via ZEC, and the agent issues orders against that balance with cryptographic signatures (e.g. BIP-322 or shielded viewing keys) rather than generating one-off QR invoices for every micro-payout.

### Issue 2: Fixed Denomination Discovery vs Custom Amount Error Ergonomics
- **The Problem:** Many products in the catalog (e.g., Steam Global or Apple) only support discrete packages ($10, $25, $50, $100), while others allow continuous ranges ($5 to $500). When an agent commands `@zool reward @user $37 Steam`, the API rejects with generic validation errors rather than suggesting the nearest valid denomination (e.g. `[25, 50]`).
- **Friction:** The agent had to implement custom client-side fuzzy rounding to prevent invoice creation failures.
- **Recommendation:** In the v2 API response, return a `suggested_denominations` array or a `step` parameter in the error payload so autonomous agents can auto-adjust without failing the purchase loop.

### Issue 3: Rate Limiting and Sandbox Credentials
- **The Problem:** In a hackathon setting where multiple subagents and automated bots test PR comment triggers concurrently, rate limits on catalog querying and invoice creation triggered 429s without explicit `Retry-After` headers.
- **Recommendation:** Implement standard RFC 7231 `Retry-After` headers and provide a sandbox mock mode header (e.g. `X-Bitrefill-Mock: true`) that simulates immediate invoice fulfillment with valid sample voucher codes for local developer unit tests.

### Issue 4: Webhook Signature Verification in Serverless Environments
- **The Problem:** Webhook payloads for order fulfillment occasionally had minor body formatting differences between local tunnels and edge runtimes, causing HMAC signature mismatches.
- **Recommendation:** Provide an official TypeScript/Node SDK helper (`@bitrefill/agent-sdk`) with built-in webhook verification, invoice polling, and error classes.

---

## 4. Suggested Features for Future Agent Hackathons
1. **MCP Native Tool Schema:**
   - An official Model Context Protocol (MCP) server that exports high-level tool definitions like `search_catalog({ country, category })` and `buy_gift_card({ brand, amount, recipient })` with built-in guardrails (max budget parameter).
2. **Zcash Unified Address & Shielded Memo Support:**
   - Allowing invoices to generate unified addresses with embedded order IDs directly in the shielded memo field for automated daemon sweeping.

---

## 5. Conclusion
Bitrefill's infrastructure is uniquely positioned to bridge crypto rails with real-world utility. Providing agent-first APIs with pre-funded pool limits and smart denomination rounding will make autonomous commerce agents radically simpler to build and deploy.
