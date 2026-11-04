# Z-Split: Privacy and Trust Note
**Free Society Hack — Mumbai, 4–5 November 2026**
**Track**: Privacy Track (Zcash)

---

## 1. One-Line Pitch
**Z-Split** is a non-custodial, reserve-first group payout protocol allowing organizers to deposit shielded Zcash (TAZ) once and disburse private payouts to up to 50 recipients via one-time cryptographically hashed bearer tickets with zero on-chain linkability and FROST threshold treasury signing.

---

## 2. Verified Testnet Shielded Transaction ID
Stadium / Explorer Verified:
- **Transaction ID**: [`c271dac34b29582a826878ee7b586b6c6d38ae385283d6be764da3ee089732bb`](https://testnet.zcashexplorer.app/tx/c271dac34b29582a826878ee7b586b6c6d38ae385283d6be764da3ee089732bb)
- **Network**: Zcash Testnet (NU7, Ironwood pool)
- **Upstream Lightwalletd**: `https://testnet.zec.rocks:443`

---

## 3. Four Core Questions

### A. What stays private?
1. **Recipient Identity & Personal Wallets**:
   - The coordinator backend **never receives, logs, or stores** recipient spending keys, seeds, or permanent personal destination addresses.
   - Payout redemption occurs into client-generated ephemeral shielded addresses (`utest1...`), which the client sweeps locally.
2. **Transaction Linkability**:
   - Claim bearer tokens are isolated exclusively in the URL fragment (`#t=TOKEN`). Per RFC 3986, URL fragments are never transmitted to web servers in HTTP requests.
   - When submitted during redemption, tokens are compared in constant time against pre-computed keyed HMAC-SHA256 digests. Raw tokens are never logged or stored in PostgreSQL.
3. **In-Flight Destination Protection**:
   - The recipient's destination address is encrypted in-memory using ChaCha20-Poly1305 AEAD bound to `(claim_id, ephemeral_address)`. The plaintext receiver is only momentarily held in an isolated non-blocking proving worker during note construction.
4. **On-Chain Shielded Invariants**:
   - Under NU7 and NU6.3 (Ironwood pool), sender, receiver, and payout amounts remain completely shielded on-chain.
5. **Threshold Treasury Governance**:
   - Integrated with official **Zcash Foundation FROST** (`frost-ed25519` v3.0.0). Payout authorization requires a 2-of-3 threshold signature from co-organizers without any party ever reconstructing the aggregate group secret key. On-chain and over the wire, the aggregated threshold signature looks indistinguishable from a standard single Schnorr signature.

---

### B. What does your app reveal, and to whom?
1. **Lightwalletd Server (`testnet.zec.rocks:443`)**:
   - Sees the coordinator server's IP address and gRPC sync block queries.
   - Sees shielded transaction broadcast blobs.
2. **Z-Split Coordinator Server**:
   - Knows the room total budget, claim count, and expiration schedule.
   - Temporarily sees the recipient's ephemeral shielded receiving address in the encrypted redemption payload before it is submitted to the worker.
   - Sees client IP addresses accessing HTTP endpoints (mitigated by stripped headers `x-forwarded-for` and bounded in-memory rate limiting).
3. **On-Chain Observers**:
   - Can see the timestamp and block height of deposit and disbursement transactions.
   - Transaction graph analysis cannot link the organizer's funding deposit to the recipient's personal sweep address.

---

### C. What do users have to trust?
1. **The Coordinator Operator**:
   - Users trust that the coordinator accurately honors one-time claim reservations and executes the shielded payout job upon token redemption before room expiration.
2. **Upstream Lightwalletd Server**:
   - Trusted not to censor transaction broadcasts or serve deceptive block headers.
3. **Client Integrity**:
   - The recipient trusts their own browser or mobile wallet (Zingo, Zodl) to securely retain their spending key and sweep their funds.

---

### D. What would you fix next?
1. **Network Layer Privacy (Nym Integration)**:
   - Route all coordinator gRPC lightwalletd synchronization and transaction broadcasts through the **Nym Mixnet** to mask the coordinator's server IP and sync patterns.
2. **Client-Side ZK Proving via WebZjs / WebAssembly**:
   - Move note generation and proving into the browser/mobile client so that even ephemeral destination addresses never leave the client device.
3. **Threshold DKG in the Browser**:
   - Enable peer-to-peer distributed key generation (DKG) among co-organizers using WebRTC before pool funding.
