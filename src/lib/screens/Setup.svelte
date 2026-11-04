<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    app,
    createRealRoom,
    saveRoom,
    notify,
    checkDepositStatus,
    mintClaims,
    copyText,
  } from '#lib/state.svelte.ts';
  import { poolTotal, formatZatToZec } from '#lib/model.ts';
  import { setupBackendFrost, signBackendFrost, devConfirmBackendDeposit } from '#lib/api.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import QR from '#lib/components/QR.svelte';
  import { enter } from '#lib/motion.ts';

  let { fund = false }: { fund?: boolean } = $props();
  let name = $state('');
  let count = $state(3);
  let amount = $state(0.5);
  let expiry = $state(7);
  let split = $state<'tiers'>('tiers');
  let giftCards = $state(true);
  let busy = $state(false);
  let checking = $state(false);
  let error = $state('');
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let frostEnabled = $state(false);
  let frostBusy = $state(false);
  let fundedSuccess = $state(false);

  // Results & Distribution: 1st, 2nd, 3rd positions (email & prize money)
  let resultsMode = $state<'manual' | 'file'>('manual');
  let uploadedFileName = $state('');
  let recipients = $state([
    { rank: '1st Place', email: 'alice@zkproof.org', prize: 1.5, required: true },
    { rank: '2nd Place', email: 'bob@cryptomail.io', prize: 1.0, required: false },
    { rank: '3rd Place', email: 'charlie@privacy.net', prize: 0.5, required: false },
  ]);

  function addRecipient() {
    const nextIdx = recipients.length + 1;
    recipients = [
      ...recipients,
      { rank: `${nextIdx}th Place`, email: '', prize: Number(amount) || 0.5, required: false }
    ];
    if (recipients.length > count) {
      count = recipients.length;
    }
  }

  function removeRecipient(idx: number) {
    if (idx === 0) return; // 1st place is mandatory
    recipients = recipients.filter((_, i) => i !== idx);
  }

  function handleFileUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (file) {
      uploadedFileName = file.name;
      notify(`Parsed ${file.name}: 1st, 2nd, 3rd positions imported.`);
    }
  }
  const total = $derived(
    (() => {
      const explicitSum = recipients.reduce((s, r) => s + (Number(r.prize) || 0), 0);
      const remainingClaims = Math.max(0, (Number(count) || 1) - recipients.length);
      const remainingSum = remainingClaims * (Number(amount) || 0);
      return explicitSum + remainingSum;
    })()
  );

  const payoutTotalZat = $derived(
    Math.round(total * 100_000_000)
  );
  const feeReserveTotalZat = $derived(
    app.backendRoom?.fee_budget_per_ticket_zat
      ? app.backendRoom.fee_budget_per_ticket_zat * (Number(count) || 1)
      : 10_000 * (Number(count) || 1)
  );
  const requiredZat = $derived(
    app.depositRequiredZat || (payoutTotalZat + feeReserveTotalZat)
  );
  const cleanDepositAmount = $derived(formatZatToZec(requiredZat));
  const cleanPayoutAmount = $derived(formatZatToZec(payoutTotalZat));
  const cleanFeeAmount = $derived(formatZatToZec(feeReserveTotalZat));

  const zip321Uri = $derived(
    app.depositAddress
      ? `zcash:${app.depositAddress}?amount=${cleanDepositAmount}&message=${encodeURIComponent('Zool Pool ' + (app.backendRoom?.id || '').slice(0, 8))}`
      : ''
  );

  async function handleFrostSetup() {
    frostBusy = true;
    try {
      const res = await setupBackendFrost();
      app.frostSetup = {
        min_signers: res.min_signers,
        max_signers: res.max_signers,
        group_public_key: res.group_public_key,
        public_key_package: res.public_key_package,
        key_packages: res.key_packages,
        authorized: false,
      };
      frostEnabled = true;
      notify('FROST 2-of-3 threshold treasury initialized!');
    } catch (e: any) {
      notify(`FROST setup failed: ${e.message || 'error'}`);
    } finally {
      frostBusy = false;
    }
  }

  async function handleFrostSign() {
    if (!app.frostSetup) return;
    frostBusy = true;
    try {
      const msg = `zsplit:authorize-claims:${app.backendRoom?.id || 'demo'}:${count}`;
      const signers = [app.frostSetup.key_packages[0], app.frostSetup.key_packages[1]];
      const res = await signBackendFrost(
        app.frostSetup.group_public_key,
        msg,
        signers,
        app.frostSetup.public_key_package
      );
      if (res.verified) {
        app.frostSetup.authorized = true;
        app.frostSetup.signature_hex = res.signature_hex;
        notify('FROST 2-of-3 threshold signature verified!');
      }
    } catch (e: any) {
      notify(`FROST signing error: ${e.message || 'error'}`);
    } finally {
      frostBusy = false;
    }
  }

  onMount(() => {
    if (fund && app.backendRoom?.id) {
      void checkDepositStatus();
      pollInterval = setInterval(async () => {
        if (!app.room.funded) {
          await checkDepositStatus();
        }
      }, 3500);
    }
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  async function refreshDeposit() {
    checking = true;
    const res = await checkDepositStatus();
    checking = false;
    if (res && res.confirmed_zat >= app.depositRequiredZat && app.depositRequiredZat > 0) {
      notify('Shielded deposit confirmed on testnet! You can now mint your tickets.');
    } else {
      notify(`Status checked: ${(app.depositConfirmedZat / 100_000_000).toFixed(4)} ZEC confirmed.`);
    }
  }

  async function simulateFunding() {
    checking = true;
    try {
      if (app.backendRoom?.id) {
        const res = await devConfirmBackendDeposit(app.backendRoom.id);
        app.depositConfirmedZat = res.confirmed_zat;
        app.room.funded = true;
        saveRoom();
        notify('Testnet deposit confirmed on backend! Ready to mint claim tickets.');
      } else {
        app.depositConfirmedZat = app.depositRequiredZat || 5_100_000;
        app.room.funded = true;
        saveRoom();
        notify('Testnet deposit confirmed locally. Ready to mint claim tickets.');
      }
    } catch (e: any) {
      notify(`Confirmation failed: ${e.message}`);
    } finally {
      checking = false;
    }
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    if (!fund) {
      if (!name.trim()) {
        error = 'Give your room a name so people know who paid them.';
        document.getElementById('room-name')?.focus();
        return;
      }
      if (resultsMode === 'manual') {
        const first = recipients[0];
        if (!first.email.trim() || !first.email.includes('@')) {
          error = '1st place winner email is mandatory and must be valid.';
          return;
        }
        if (!first.prize || Number(first.prize) <= 0) {
          error = '1st place prize money must be greater than 0 ZEC.';
          return;
        }
      }
      if (
        !Number.isInteger(Number(count)) ||
        count < 1 ||
        count > 100 ||
        amount < 0.0001 ||
        amount > 1000
      ) {
        error = 'Use 1–100 claims and an amount between 0.0001 and 1,000 ZEC.';
        return;
      }
      const values = {
        name: name.trim(),
        count: Number(count),
        amount: Number(amount),
        expiry: Number(expiry),
        split,
        giftCards,
      };

      busy = true;
      try {
        const amounts = recipients.map((r) =>
          Math.max(10_000, Math.round((Number(r.prize) || 0.001) * 100_000_000))
        );
        while (amounts.length < count) {
          amounts.push(Math.max(10_000, Math.round((Number(amount) || 0.001) * 100_000_000)));
        }
        await createRealRoom(values, amounts);
        await goto('/room/fund');
      } catch (err: any) {
        error = err.message || 'Failed to create room on backend';
      } finally {
        busy = false;
      }
    } else {
      busy = true;
      try {
        if (app.backendRoom?.id) {
          await mintClaims();
          notify('Claim tickets minted on Zcash backend!');
        } else {
          notify('Demo pool funded. Your one-time links are ready.');
        }
        await goto('/room/claims');
      } catch (err: any) {
        error = err.message || 'Failed to mint claims from backend';
      } finally {
        busy = false;
      }
    }
  }

  $effect(() => {
    if (app.ready && !name && (fund || !app.room.funded)) {
      name = app.room.name;
      count = app.room.count || 5;
      amount = app.room.amount;
      expiry = app.room.expiry;
      split = 'tiers';
      giftCards = app.room.giftCards;
    }
  });
</script>

<svelte:head><title>{fund ? 'Fund your room' : 'Set up the room'} — Zool</title></svelte:head>
<main id="main" class="workspace setup-page" use:enter>
  <form class="setup-grid" onsubmit={submit}>
    <section class="setup-fields">
      <p class="hand intro">
        {fund ? 'a little pool. a whole room of possibilities.' : 'first, name it, size it & add results'}
      </p>
      <h1>{fund ? 'FUND THE ROOM.' : 'SET UP THE ROOM.'}</h1>
      {#if fund}
        <div class="fund-panel">
          {#if fundedSuccess || app.room.funded}
            <div class="fund-success-coin" use:enter={'art'}>
              <Art kind="coin-check" />
              <p class="hand success-label">Payment verified & locked in pool!</p>
            </div>
          {/if}
          <div class="fund-header">
            <h2>COORDINATOR SHIELDED POOL</h2>
            <p class="hand">
              Send <strong>{cleanDepositAmount} TAZ</strong> from your <strong>Zingo / Zodl wallet</strong> to fund this room's {count} one-time claim links.
            </p>
          </div>

          {#if app.depositAddress}
            <div class="qr-deposit-box">
              <QR value={zip321Uri} label="Coordinator Zcash Testnet Shielded Deposit QR" />
              <div class="deposit-meta">
                <span class="deposit-tag">ZCASH TESTNET (ORCHARD / IRONWOOD)</span>
                <p class="hand deposit-instructions">
                  📱 <strong>In Zingo / Zodl Wallet:</strong> Tap Send → Scan QR code (clean ZIP-321 amount).
                </p>
                <div class="addr-box">
                  <code>{app.depositAddress}</code>
                  <button
                    type="button"
                    class="pill copy-btn"
                    onclick={() => copyText(app.depositAddress, 'Deposit address copied!')}
                  >
                    Copy Address
                  </button>
                </div>
              </div>
            </div>
          {/if}

          <div class="live-status-card" class:funded={app.room.funded}>
            <div class="status-indicator">
              <span class="status-dot" class:online={app.room.funded}></span>
              <strong>{app.room.funded ? '✓ POOL FUNDED & CONFIRMED' : '⏳ Awaiting Shielded Deposit on Chain'}</strong>
            </div>
            <div class="status-metrics">
              <div>
                <span class="muted">Confirmed:</span>
                <strong>{formatZatToZec(app.depositConfirmedZat)} TAZ</strong>
              </div>
              <div>
                <span class="muted">Required:</span>
                <strong>{cleanDepositAmount} TAZ</strong>
              </div>
            </div>
            <div class="status-actions">
              <button type="button" class="text-link" disabled={checking} onclick={refreshDeposit}>
                {checking ? 'Checking…' : '↻ Refresh Status'}
              </button>
              {#if !app.room.funded}
                <button type="button" class="text-link dev-shortcut" onclick={simulateFunding}>
                  ⚡ Confirm Instantly (Dev)
                </button>
              {/if}
            </div>
          </div>

          <div class="frost-threshold-card" style="margin-top: 1rem; padding: 1rem; background: var(--bg-surface, #1e1e24); border: 1px solid var(--border-color, #333); border-radius: 8px;">
            <div class="frost-header" style="display: flex; align-items: center; justify-content: space-between;">
              <span class="deposit-tag">🛡️ ZCASH FOUNDATION FROST</span>
              <strong>2-of-3 Threshold Signing</strong>
            </div>
            <p class="hand" style="font-size: 0.85rem; margin: 0.5rem 0;">
              Distribute coordinator keys across 3 treasury co-organizers. Requires 2 valid Schnorr signature shares to authorize batch payout.
            </p>
            {#if !app.frostSetup}
              <button
                type="button"
                class="pill"
                style="margin-top: 0.5rem;"
                disabled={frostBusy}
                onclick={handleFrostSetup}
              >
                {frostBusy ? 'Initializing FROST…' : '🛡️ Enable 2-of-3 FROST Treasury'}
              </button>
            {:else}
              <div class="frost-details" style="font-size: 0.85rem; display: flex; flex-direction: column; gap: 0.5rem;">
                <div>
                  <span class="muted">Group Public Key:</span><br/>
                  <code style="word-break: break-all; font-size: 0.75rem;">{app.frostSetup.group_public_key}</code>
                </div>
                <div>
                  <span class="muted">Signer Threshold:</span>
                  <strong>{app.frostSetup.min_signers} of {app.frostSetup.max_signers} signers required</strong>
                </div>
                <div>
                  <span class="muted">Status:</span>
                  <strong style="color: {app.frostSetup.authorized ? '#2ecc71' : '#f39c12'}">
                    {app.frostSetup.authorized ? '✓ 2-of-3 FROST Signature Verified' : '⏳ Pending 2-of-3 Co-signer Authorization'}
                  </strong>
                </div>
                {#if !app.frostSetup.authorized}
                  <button
                    type="button"
                    class="pill"
                    style="margin-top: 0.25rem;"
                    disabled={frostBusy}
                    onclick={handleFrostSign}
                  >
                    {frostBusy ? 'Aggregating Shares…' : '✍️ Co-sign Batch with 2 Signers'}
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>
        <button type="button" class="text-link" onclick={() => goto('/room/new')}
          >← Edit room details</button
        >
      {:else}
        <div class="field">
          <label for="room-name">ROOM NAME</label><input
            id="room-name"
            maxlength="80"
            placeholder="e.g. Builder bounties"
            bind:value={name}
            aria-invalid={!!error && !name.trim()}
            autocomplete="off"
          />
        </div>
        <div class="field-row">
          <div class="field">
            <label for="claim-count">WINNER POSITIONS</label>
            <div class="stepper">
              <button
                type="button"
                aria-label="One fewer claim"
                disabled={count <= 1}
                onclick={() => {
                  count = Math.max(1, Number(count) - 1);
                  if (recipients.length > count && count >= 1) {
                    recipients = recipients.slice(0, count);
                  }
                }}>−</button
              ><input
                id="claim-count"
                type="number"
                min="1"
                max="100"
                bind:value={count}
                aria-label="Number of claims"
              /><button
                type="button"
                aria-label="One more claim"
                disabled={count >= 100}
                onclick={() => {
                  count = Math.min(100, Number(count) + 1);
                  if (recipients.length < count) {
                    addRecipient();
                  }
                }}>+</button
              >
            </div>
          </div>
          <div class="field gift-field">
            <label class="gift-toggle"
              ><span class="hand">Let people cash out to a gift card</span><button
                type="button"
                role="switch"
                aria-label="Allow gift card payouts"
                aria-checked={giftCards}
                class="switch"
                class:on={giftCards}
                onclick={() => (giftCards = !giftCards)}><span></span></button
              ></label
            >
          </div>
        </div>
        <div class="results-section">
          <div class="results-header">
            <div>
              <span class="label">RESULTS & DISTRIBUTION</span>
              <p class="hand results-kicker">
                1st, 2nd, and 3rd place positions with their respective prize rewards. Enter winner emails to dispatch claims.
              </p>
            </div>
            <div class="results-mode-pills">
              <button
                type="button"
                class="pill"
                class:selected={resultsMode === 'manual'}
                onclick={() => (resultsMode = 'manual')}
              >
                WRITE RESULTS
              </button>
              <button
                type="button"
                class="pill"
                class:selected={resultsMode === 'file'}
                onclick={() => (resultsMode = 'file')}
              >
                UPLOAD FILE
              </button>
            </div>
          </div>

          {#if resultsMode === 'manual'}
            <div class="recipients-list">
              {#each recipients as recipient, idx}
                <div class="recipient-row" class:mandatory={recipient.required}>
                  <div class="recipient-rank">
                    <span class="rank-badge" class:gold={idx === 0}>{recipient.rank}</span>
                    {#if recipient.required}
                      <span class="req-tag">MANDATORY</span>
                    {/if}
                    <span class="muted hand rank-note">
                      {idx === 0 ? '1st prize reward' : idx === 1 ? '2nd prize reward' : idx === 2 ? '3rd prize reward' : 'Position prize'}
                    </span>
                  </div>
                  <div class="recipient-inputs">
                    <div class="email-input-wrapper">
                      <input
                        type="email"
                        placeholder="winner@email.com"
                        bind:value={recipient.email}
                        required={recipient.required}
                        aria-label={`${recipient.rank} Email`}
                      />
                    </div>
                    <div class="prize-input-wrapper">
                      <input
                        type="number"
                        step="0.01"
                        min="0.01"
                        max="1000"
                        placeholder="Prize"
                        bind:value={recipient.prize}
                        required
                        aria-label={`${recipient.rank} Prize Amount in ZEC`}
                      />
                      <span class="currency-tag">ZEC</span>
                    </div>
                    {#if !recipient.required}
                      <button
                        type="button"
                        class="remove-recipient-btn"
                        aria-label={`Remove ${recipient.rank}`}
                        onclick={() => removeRecipient(idx)}
                      >
                        <Icon name="close" size={14} />
                      </button>
                    {/if}
                  </div>
                </div>
              {/each}

              <div class="recipients-footer-row">
                <button type="button" class="add-recipient-btn" onclick={addRecipient}>
                  <Icon name="plus" size={16} />
                  <span>+ Add another position</span>
                </button>
              </div>
            </div>
          {:else}
            <div class="file-upload-card">
              <label class="file-dropzone" for="results-file-input">
                <Icon name="copy" size={28} />
                <span class="drop-title">Upload Results CSV / JSON</span>
                <span class="hand drop-subtitle">Columns: rank, email, prize_zec (1st place required)</span>
                {#if uploadedFileName}
                  <div class="uploaded-badge">
                    <span class="check-mini"><Icon name="check" size={14} /></span>
                    <span>{uploadedFileName}</span>
                  </div>
                {/if}
                <input
                  id="results-file-input"
                  type="file"
                  accept=".csv,.json,.txt"
                  onchange={handleFileUpload}
                  class="sr-only"
                />
              </label>
            </div>
          {/if}
        </div>
        <p class="muted hand refund-note">
          Unclaimed ZEC goes back to your pool when links expire. Unique claim links and QR codes will be dispatched to winners.
        </p>
      {/if}
      {#if error}<p class="form-error hand" role="alert">{error}</p>{/if}
    </section>
    <aside class="ticket-side">
      <div class="pool-ticket">
        <div class="ticket-coin"><Art kind="coin" /></div>
        <h2 class="label">POOL TICKET</h2>
        <dl class="hand ticket-details">
          <div>
            <dt>Positions</dt>
            <dd>{count || 0}</dd>
          </div>
          <div>
            <dt>1st Place</dt>
            <dd>{(recipients[0]?.prize || 1.5).toFixed(2)} ZEC</dd>
          </div>
          <div>
            <dt>Expires</dt>
            <dd>{expiry} days</dd>
          </div>
        </dl>
        <div class="dotted-line"></div>
        <p class="hand deposit-label">You deposit</p>
        <div class="deposit-total">{cleanDepositAmount} TAZ</div>
        <p class="muted hand fee-note">
          {cleanPayoutAmount} TAZ payouts + {cleanFeeAmount} TAZ network fee reserve
        </p>
        <Action type="submit" disabled={busy || (fund && !app.room.funded)}
          >{busy ? 'Processing…' : fund ? (app.room.funded ? 'MINT CLAIM TICKETS →' : 'Awaiting ZEC Deposit…') : 'Generate Coordinator Address →'}</Action
        >
      </div>
      <ul class="promises hand">
        <li>
          <span class="check-mini"><Icon name="check" size={14} /></span>No wallet addresses
          collected
        </li>
        <li>
          <span class="check-mini"><Icon name="check" size={14} /></span>Each link works exactly
          once
        </li>
        <li>
          <span class="check-mini"><Icon name="check" size={14} /></span>Funds stay shielded end to
          end
        </li>
      </ul>
    </aside>
  </form>
</main>

<style>
  .qr-deposit-box {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    background: #fbfbf9;
    border: 2px solid #000;
    padding: 1.25rem;
    border-radius: 8px;
    margin: 1.25rem 0;
  }
  .deposit-meta {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow: hidden;
  }
  .deposit-tag {
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.05em;
    color: #4a5568;
  }
  .addr-box {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .addr-box code {
    font-family: monospace;
    font-size: 0.75rem;
    background: #edf2f7;
    padding: 0.4rem;
    border-radius: 4px;
    word-break: break-all;
  }
  .live-status-card {
    border: 2px dashed #718096;
    padding: 1rem;
    border-radius: 8px;
    background: #fff;
    margin-bottom: 1rem;
  }
  .live-status-card.funded {
    border: 2px solid #38a169;
    background: #f0fff4;
  }
  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #e53e3e;
  }
  .status-dot.online {
    background: #38a169;
  }
  .status-metrics {
    display: flex;
    gap: 2rem;
    font-size: 0.9rem;
    margin-bottom: 0.5rem;
  }
  .status-actions {
    display: flex;
    gap: 1rem;
    align-items: center;
  }
  .dev-shortcut {
    color: #b7791f;
    font-weight: 700;
  }
</style>
