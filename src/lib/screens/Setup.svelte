<script lang="ts">
  import { goto } from '$app/navigation';
  import { app, createRoom, saveRoom, notify } from '#lib/state.svelte.ts';
  import { poolTotal } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import { enter } from '#lib/motion.ts';
  let { fund = false }: { fund?: boolean } = $props();
  let name = $state('');
  let count = $state(3);
  let amount = $state(0.5);
  let expiry = $state(7);
  let split = $state<'tiers'>('tiers');
  let giftCards = $state(true);
  let busy = $state(false);
  let fundedSuccess = $state(false);
  let error = $state('');

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

  // Computed total directly sums the positions
  const total = $derived(
    (() => {
      const explicitSum = recipients.reduce((s, r) => s + (Number(r.prize) || 0), 0);
      const remainingClaims = Math.max(0, (Number(count) || 1) - recipients.length);
      const remainingSum = remainingClaims * (Number(amount) || 0);
      return explicitSum + remainingSum;
    })()
  );

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    if (!name.trim()) {
      error = 'Give your room a name so people know who paid them.';
      document.getElementById('room-name')?.focus();
      return;
    }
    // Check 1st place recipient email is filled if manual
    if (!fund && resultsMode === 'manual') {
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
      amount < 0.01 ||
      amount > 1000
    ) {
      error = 'Use 1–100 claims and an amount between 0.01 and 1,000 ZEC.';
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
    if (!fund) {
      app.room = { ...app.room, ...values, claimed: [], gifts: [], closed: false, funded: false };
      saveRoom();
      await goto('/room/fund');
    } else {
      busy = true;
      fundedSuccess = true;
      await new Promise((r) => setTimeout(r, 1200));
      createRoom(values);
      notify('Demo pool funded. Your one-time links are ready.');
      await goto('/room/claims');
      busy = false;
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

<svelte:head><title>{fund ? 'Fund your room' : 'Set up the room'} — Z-Split</title></svelte:head>
<main id="main" class="workspace setup-page" use:enter>
  <form class="setup-grid" onsubmit={submit}>
    <section class="setup-fields">
      <p class="hand intro">
        {fund ? 'a little pool. a whole room of possibilities.' : 'first, name it, size it & add results'}
      </p>
      <h1>{fund ? 'FUND THE ROOM.' : 'SET UP THE ROOM.'}</h1>
      {#if fund}
        <div class="fund-panel">
          {#if fundedSuccess}
            <div class="fund-success-coin" use:enter={'art'}>
              <Art kind="coin-check" />
              <p class="hand success-label">Payment verified & locked in pool!</p>
            </div>
          {:else}
            <Art kind="envelope" />
          {/if}
          <h2>ONE POOL.<br />EVERYONE GETS PAID.</h2>
          <p class="hand">
            Your room is ready. Add {total.toFixed(2)} ZEC to make {count} one-time claim links.
          </p>
          <div class="demo-note">
            <span class="check-mini"><Icon name="check" size={15} /></span><span
              >You're in a demo. No wallet connection or real funds needed.</span
            >
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
                step="1"
                bind:value={count}
                required
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
        <div class="deposit-total">{total.toFixed(2)} ZEC</div>
        <p class="muted hand fee-note">
          {fund
            ? 'mock funding. real privacy-minded design.'
            : 'plus network fees, worked out when you fund'}
        </p>
        <Action type="submit" disabled={busy}
          >{busy ? 'Making your links…' : fund ? 'Fund demo pool' : 'Fund and make links'}</Action
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
