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
  let count = $state(50);
  let amount = $state(0.5);
  let expiry = $state(7);
  let split = $state<'equal' | 'tiers'>('equal');
  let giftCards = $state(true);
  let busy = $state(false);
  let error = $state('');
  const total = $derived(
    poolTotal({ ...app.room, count: Number(count) || 1, amount: Number(amount) || 0, split })
  );
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    if (!name.trim()) {
      error = 'Give your room a name so people know who paid them.';
      document.getElementById('room-name')?.focus();
      return;
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
      await new Promise((r) => setTimeout(r, 850));
      createRoom(values);
      notify('Demo pool funded. Your one-time links are ready.');
      await goto('/room/claims');
      busy = false;
    }
  }
  $effect(() => {
    if (app.ready && !name && (fund || !app.room.funded)) {
      name = app.room.name;
      count = app.room.count;
      amount = app.room.amount;
      expiry = app.room.expiry;
      split = app.room.split;
      giftCards = app.room.giftCards;
    }
  });
</script>

<svelte:head><title>{fund ? 'Fund your room' : 'Set up the room'} — Z-Split</title></svelte:head>
<main id="main" class="workspace setup-page" use:enter>
  <form class="setup-grid" onsubmit={submit}>
    <section class="setup-fields">
      <p class="hand intro">
        {fund ? 'a little pool. a whole room of possibilities.' : 'first, name it and size it'}
      </p>
      <h1>{fund ? 'FUND THE ROOM.' : 'SET UP THE ROOM.'}</h1>
      {#if fund}
        <div class="fund-panel">
          <Art kind="envelope" />
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
            <label for="claim-count">HOW MANY CLAIM LINKS</label>
            <div class="stepper">
              <button
                type="button"
                aria-label="One fewer claim"
                disabled={count <= 1}
                onclick={() => (count = Math.max(1, Number(count) - 1))}>−</button
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
                onclick={() => (count = Math.min(100, Number(count) + 1))}>+</button
              >
            </div>
          </div>
          <div class="field">
            <label for="claim-amount">ZEC PER CLAIM</label>
            <div class="amount-input">
              <input
                id="claim-amount"
                type="number"
                step="0.01"
                min="0.01"
                max="1000"
                bind:value={amount}
                required
              /><span>ZEC</span>
            </div>
          </div>
        </div>
        <fieldset class="split-field">
          <legend>SPLIT</legend>
          <div class="split-buttons">
            <button
              type="button"
              class="pill"
              class:selected={split === 'equal'}
              aria-pressed={split === 'equal'}
              onclick={() => (split = 'equal')}>SAME FOR EVERYONE</button
            ><button
              type="button"
              class="pill"
              class:selected={split === 'tiers'}
              aria-pressed={split === 'tiers'}
              onclick={() => (split = 'tiers')}>TIERS: 1ST, 2ND,<br />3RD</button
            >
          </div>
          {#if split === 'tiers'}<p class="field-note hand">
              1st gets 3×. 2nd and 3rd get 2×. Everyone else gets the base amount.
            </p>{/if}
        </fieldset>
        <div class="expiry-row">
          <div class="field">
            <label for="expiry">LINKS EXPIRE AFTER</label><select id="expiry" bind:value={expiry}
              >{#each [1, 3, 7, 14, 30] as day}<option value={day}
                  >{day} {day === 1 ? 'day' : 'days'}</option
                >{/each}</select
            >
          </div>
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
        <p class="muted hand refund-note">
          Unclaimed ZEC goes back to your pool when links expire.
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
            <dt>Claim links</dt>
            <dd>{count || 0}</dd>
          </div>
          <div>
            <dt>{split === 'equal' ? 'Per claim' : 'Base claim'}</dt>
            <dd>{(Number(amount) || 0).toFixed(2)} ZEC</dd>
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
