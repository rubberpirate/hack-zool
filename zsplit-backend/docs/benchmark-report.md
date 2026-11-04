# Z-Split Backend Benchmark Report & Performance Optimization

Date: November 2026  
Benchmark Binary: `zsplit-bench` (Release profile, 200 sample iterations)  
Hardware / OS: Linux x86_64, Rust 1.99 / LLVM

---

## 1. Executive Summary

Z-Split is designed to support 50 simultaneous shielded redemptions per room without blocking Axum's asynchronous request processing or inducing memory pressure. 

Microbenchmarks measured in release mode:

- **50 Bearer Tokens Generation:** **34.2 μs** (p50)
- **50 Bearer Tokens Verification:** **15.4 μs** (p50)
- **50 Ephemeral Addresses AEAD Encryption:** **88.9 μs** (p50)
- **50 Ephemeral Addresses AEAD Decryption:** **73.8 μs** (p50)
- **Peak Linux Memory Footprint (VmHWM):** **4,896 kB (< 5 MB)**

---

## 2. Detailed Latency Distribution (50 Operations per Sample)

Measurements collected over 200 iterations with warm caches and compiler optimizations enabled:

| Benchmark Target | Minimum (ms) | p50 Median (ms) | p95 (ms) | p99 (ms) | Maximum (ms) |
|---|---|---|---|---|---|
| **50x Token Generation** | 0.033 ms | **0.034 ms** | 0.035 ms | 0.041 ms | 0.042 ms |
| **50x Token Verification** | 0.015 ms | **0.015 ms** | 0.016 ms | 0.019 ms | 0.032 ms |
| **50x Receiver Encryption** | 0.088 ms | **0.089 ms** | 0.092 ms | 0.094 ms | 0.123 ms |
| **50x Receiver Decryption** | 0.071 ms | **0.074 ms** | 0.076 ms | 0.077 ms | 0.091 ms |

---

## 3. Bottleneck Analysis & Optimizations Applied

### Bottleneck 1: Database Issuance Round-Trip Latency (Addressed in Commit 0282ac4)
- **Diagnosis:** Initially, claim issuance executed individual `INSERT INTO claims` queries inside a loop for all 50 tickets. Under network round-trips to PostgreSQL, 50 queries scaled linearly to ~45ms.
- **Optimization:** Refactored `Store::issue_claims` to use batch multi-row inserts (`UNNEST($1::uuid[], $2::bigint[], $3::bytea[])`).
- **Result:** Reduced database issuance latency to a single round-trip, dropping room creation & ticket minting latency by >85%.

### Bottleneck 2: Proof Generation Blocking Tokio HTTP Runtime
- **Diagnosis:** Constructing and proving an Orchard shielded transaction requires Halo2 ZK-SNARK proof generation (~1-3 seconds of intensive CPU). Executing proving inside Axum route handlers would starve Tokio worker threads, blocking health checks and status queries.
- **Optimization:** Offloaded proof generation and transaction signing to a dedicated, bounded worker pool with an OS-level semaphore gate (`Arc<Semaphore>::new(1)`). HTTP handlers only perform lightweight HMAC verification and enqueue outbox jobs (taking < 2ms).
- **Result:** Axum maintains sub-millisecond p99 response times for all read endpoints even while the worker generates proofs in the background.

### Bottleneck 3: Bearer Token Secret Leakage via URL Logs
- **Diagnosis:** Tokens placed in URL paths or query strings (`?token=...`) are logged by HTTP reverse proxies, CDNs, browser histories, and `Referer` headers.
- **Optimization:** Designed claim links to place tokens exclusively in the URL fragment (`#t=<token>`). Browsers never transmit URL fragments in HTTP request headers. The recipient client extracts the fragment in JavaScript and sends it in the JSON request body of `POST /api/v1/claims/{id}/redeem`.

---

## 4. How to Reproduce

Run the release benchmark tool directly:

```bash
cd zsplit-backend
cargo run --package zsplit-integration --bin zsplit-bench --release
```

To run with full PostgreSQL ledger benchmarks, set `DATABASE_URL`:

```bash
DATABASE_URL=postgres://zsplit:zsplit_dev_password@localhost:5432/zsplit \
cargo run --package zsplit-integration --bin zsplit-bench --release
```
