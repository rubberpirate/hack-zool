<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { app, saveRoom, notify } from '#lib/state.svelte.ts';
  import { getBackendSummary } from '#lib/api.ts';
  import { remainingTotal, liveDemoClaims, daysLeft } from '#lib/model.ts';
  import Counter from '#lib/components/Counter.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import Action from '#lib/components/Action.svelte';
  import { enter } from '#lib/motion.ts';

  let closeDialog: HTMLDialogElement;
  let livePollTimer: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    if (app.backendRoom?.id) {
      const poll = async () => {
        try {
          const sum = await getBackendSummary(app.backendRoom!.id);
          app.summary = sum;
        } catch (e) {
          console.warn('Could not poll room summary:', e);
        }
      };
      void poll();
      livePollTimer = setInterval(poll, 3000);
    }
  });

  onDestroy(() => {
    if (livePollTimer) clearInterval(livePollTimer);
  });

  $effect(() => {
    if (!app.ready) return;
    if (!app.room.funded) {
      void goto('/room/fund', { replace: true });
      return;
    }
    if (!app.room.closed && daysLeft(app.room) === 0) {
      app.room.closed = true;
      saveRoom();
    }
  });

  const count = $derived(
    app.summary
      ? app.summary.funded_confirmed_count + app.summary.pending_count
      : app.room.claimed.length
  );
  const remaining = $derived(
    app.summary
      ? app.summary.spendable_zat / 100_000_000
      : remainingTotal(app.room)
  );

  $effect(() => {
    if (!app.backendRoom && app.ready && app.room.funded && app.room.id === 'demo' && !app.room.closed && count < 37) {
      const timer = setTimeout(() => {
        app.room.claimed = [...new Set([...app.room.claimed, ...liveDemoClaims])];
        app.room.gifts = [2, 7, 11, 16, 23, 30];
        saveRoom();
      }, 450);
      return () => clearTimeout(timer);
    }
  });

  function closeRoom() {
    app.room.closed = true;
    saveRoom();
    closeDialog.close();
    notify(`${remaining.toFixed(4)} ZEC remaining in coordinator pool. Room closed.`);
  }
</script>

<svelte:head><title>{count} of {app.room.count} claimed — Zool</title></svelte:head>
<main id="main" class="workspace live-page" use:enter>
  <div class="live-grid">
    <section>
      <p class="hand intro">
        room: {app.room.name.toLowerCase()}, {app.room.closed ? 'closed' : 'live'}
      </p>
      <h1 class="live-heading">
        <Counter value={count} /> OF {app.room.count}
        <mark>{app.room.closed ? 'CLOSED.' : 'CLAIMED.'}</mark>
      </h1>
      <div class="claim-dots" role="img" aria-label={`${count} of ${app.room.count} links claimed`}>
        {#each Array.from({ length: app.room.count }, (_, i) => i + 1) as n}<a
            href={`/claim/${app.room.id}-${n}`}
            class="claim-dot"
            class:used={app.room.claimed.includes(n)}
            aria-label={`Claim ${n}: ${app.room.claimed.includes(n) ? 'claimed' : 'waiting'}`}
            >{#if app.room.claimed.includes(n)}<Icon name="check" size={32} stroke={2.6} />{/if}</a
          >{/each}
      </div>
      <p class="hand privacy-note">
        we only see that a link was used. never who used it, never where the money went.
      </p>
    </section>
    <aside class="live-side">
      <div class="where-card">
        <h2 class="label">WHERE THEY WENT</h2>
        <dl>
          <div>
            <dt><Counter value={count - app.room.gifts.length} /></dt>
            <dd class="hand">swept to their own shielded<br />wallets</dd>
          </div>
          <div>
            <dt><Counter value={app.room.gifts.length} /></dt>
            <dd class="hand">cashed out to gift cards</dd>
          </div>
          <div>
            <dt><Counter value={app.room.count - count} /></dt>
            <dd class="hand">
              {app.room.closed ? 'refunded to your pool' : 'still waiting to be scanned'}
            </dd>
          </div>
        </dl>
      </div>
      <div class="remaining-card">
        <div>
          <h2 class="label">{app.room.closed ? 'REFUNDED TO YOU' : 'LEFT IN THE POOL'}</h2>
          <strong>{remaining.toFixed(2)} ZEC</strong>
        </div>
        <p class="hand">
          {app.room.closed ? 'all wrapped up.' : 'links expire'}<br />{app.room.closed
            ? 'nicely done.'
            : `in ${daysLeft(app.room)} days`}
        </p>
      </div>
      <div class="live-actions">
        <Action href="/room/claims">Show QR sheet again</Action>{#if !app.room.closed}<Action
            secondary
            cap={false}
            onclick={() => closeDialog.showModal()}>Close room and refund pool</Action
          >{:else}<Action href="/room/new" secondary cap={false}>Start another room</Action>{/if}
      </div>
    </aside>
  </div>
</main>
<dialog bind:this={closeDialog} class="confirm-dialog">
  <form method="dialog">
    <button class="dialog-x circle-button" aria-label="Cancel closing room"
      ><Icon name="close" /></button
    >
    <p class="hand">wrap things up?</p>
    <h2>CLOSE THE ROOM.</h2>
    <p class="hand">
      {app.room.count - count} links are still open. Closing the room will switch them off and refund
      {remaining.toFixed(2)} ZEC to your demo pool.
    </p>
    <Action onclick={closeRoom}>Close and refund</Action><button class="text-link" type="submit"
      >Keep the room open</button
    >
  </form>
</dialog>
