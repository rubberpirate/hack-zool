# Payout status and recovery

The receipt polls the authenticated status endpoint without caching. It shows queued → preparing → broadcast → mined → policy-confirmed states, actual transaction IDs/confirmation counts, and a visible refresh error with retry. Reorganizations and manual review are not presented as success. A local sweep report is distinct from funding confirmation.

## Proving prerequisites

The SDK adapter uses the official LocalTxProver and needs both public Sapling parameter files even for Orchard/Ironwood-only transfers. From the backend directory:

```sh
python3 scripts/download-proving-params.py
```

The script verifies exact file sizes and BLAKE2b hashes from pinned `zcash_proofs` 0.31.0. Configure `SAPLING_SPEND_PARAMS` and `SAPLING_OUTPUT_PARAMS` as shown in `.env.example`. Restart the API after changing its environment. These are public parameters; do not replace the coordinator seed or native wallet database.

## Safely recover a previously paused intent

Stop the combined API process gracefully, then run with explicit payout UUIDs:

```sh
cargo run -p zsplit-worker --example recover-payouts -- PAYOUT_UUID
```

The tool loads backend `.env`, opens the same native wallet and acquires its exclusive process lock. It refuses any payout with a native intent marker or prepared transaction, any signed PostgreSQL transaction, or a released reservation. It requeues only a manual-review intent that the native journal proves was never constructed, preserving its payout UUID, claim UUID, encrypted receiver, reservation and audit trail. Restart the API afterward.

Do not clear native intent markers, replace transaction bytes, reset a redeemed claim, or create a new payout ID to bypass a paused payment. A construction or broadcast of uncertain outcome requires native/chain reconciliation, not this recovery command.

## Temporary waits

After an earlier payout spends a note, its change can be temporarily unspendable until the configured confirmation depth. The worker defers such a job only after the native journal proves it has no construction intent. It retains the same queue entry and encrypted receiver and retries with bounded backoff. Interrupted proving and ambiguous broadcasts remain quarantined or retry only identical signed bytes.

## Incident resolved on 2026-11-04

Three claimed tickets were in `manual_review` while the UI fetched status once and always displayed a queued receipt with a broadcast checkmark. The running configuration omitted proving parameters. The native journal contained no intent for those three payout IDs. After verified parameter installation and journal-guarded recovery, real transaction construction resumed. Consult the database/native wallet for current confirmation evidence; UI tests use isolated synthetic fixtures and do not establish chain success.

All three existing claims subsequently reached `funded_confirmed` in the native worker's testnet reconciliation:

| Amount (zat) | Transaction ID | Observed confirmations |
| --- | --- | --- |
| 30,000 | `64b4a4f0f31eed1c0d043c2300977cac22472eb4ca28b2a384cdbf494c189afd` | 18 |
| 50,000 | `00e772b71550d9fd212f0d7a6d49b84dfc95da80134f7f636fda005d28c3f16e` | 11 |
| 80,000 | `9a87ab4b87abf4276f535614952c8370d23650e438859b8b237d983a777c8e8b` | 23 |

After restarting the updated worker, its room snapshot showed zero remaining confirmed/spendable balance and zero reserved balance. Balance updates now retain current SDK amounts rather than historical maxima. These observations establish coordinator funding confirmation; they do not verify a recipient's local sweep.

Validation: Rust formatting, workspace Clippy with warnings denied, workspace tests, 12 explicit PostgreSQL integration tests, Svelte type checking, 13 frontend unit tests and the production frontend build passed. The browser suite passed 13 of 18 tests, including both new payout-status tests and the redemption regression. Five older landing/demo tests still fail on missing controls/selectors (`.scene-toy`, old landing links, and `Fund and make links`); those were not changed for this fix. No new performance benchmarks, security audit or live chain-reorganization experiment was performed for this incident.
