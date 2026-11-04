<script lang="ts">
  import { app } from '#lib/state.svelte.ts';
  import { daysLeft, poolTotal, remainingTotal } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import { enter } from '#lib/motion.ts';
  const status = $derived(
    !app.room.funded
      ? 'DRAFT'
      : app.room.closed
        ? 'CLOSED'
        : daysLeft(app.room) === 0
          ? 'EXPIRED'
          : 'LIVE'
  );
</script>

<svelte:head><title>Your room — Zool</title></svelte:head>
<main id="main" class="workspace rooms-page" use:enter>
  <div class="page-heading-row">
    <div>
      <p class="hand intro">a little home for the people you’re paying.</p>
      <h1>YOUR ROOM.</h1>
    </div>
    <Action href="/room/new">Make a new room</Action>
  </div>
  <p class="page-introduction">
    Pick up where you left off. Your current room is saved in this browser.
  </p>
  <article class="room-overview">
    <div class="room-overview-main">
      <div class="room-status-row">
        <span class="status-chip" class:room-closed={status === 'CLOSED' || status === 'EXPIRED'}
          >{status}</span
        >{#if app.room.id === 'demo'}<span class="label muted">SAMPLE ROOM</span>{/if}
      </div>
      <h2>{app.room.name}</h2>
      <p class="hand">
        {app.room.count} one-time links. {app.room.split === 'equal'
          ? 'Same amount for everyone.'
          : 'A little extra for the podium.'}
      </p>
      <div
        class="room-progress"
        role="progressbar"
        aria-label="Claims redeemed"
        aria-valuenow={app.room.claimed.length}
        aria-valuemin={0}
        aria-valuemax={app.room.count}
      >
        <span style={`transform:scaleX(${app.room.claimed.length / app.room.count})`}></span>
      </div>
      <div class="room-progress-copy">
        <strong>{app.room.claimed.length} of {app.room.count} claimed</strong><span
          >{status === 'LIVE'
            ? `${daysLeft(app.room)} days left`
            : status === 'DRAFT'
              ? 'Ready when you are'
              : 'All wrapped up'}</span
        >
      </div>
      <div class="room-overview-actions">
        {#if status === 'DRAFT'}<Action href="/room/fund">Continue funding</Action>{:else}<Action
            href="/room/live">Open live room</Action
          ><Action href="/room/claims" secondary cap={false}>View claim sheet</Action>{/if}
      </div>
    </div>
    <aside class="room-summary">
      <Art kind={status === 'CLOSED' ? 'cloud' : 'coin'} />
      <dl>
        <div>
          <dt>Pool total</dt>
          <dd>{poolTotal(app.room).toFixed(2)} ZEC</dd>
        </div>
        <div>
          <dt>{status === 'CLOSED' || status === 'EXPIRED' ? 'Returned to pool' : 'Unclaimed'}</dt>
          <dd>{remainingTotal(app.room).toFixed(2)} ZEC</dd>
        </div>
        <div>
          <dt>Gift cards</dt>
          <dd>{app.room.giftCards ? 'Available' : 'Off'}</dd>
        </div>
      </dl>
    </aside>
  </article>
  <div class="room-help-strip">
    <Icon name="shield" size={28} />
    <p>Just the room’s progress. No names or wallet addresses.</p>
    <a href="/privacy" class="inline-link">How privacy works <Icon name="right" size={18} /></a>
  </div>
  <div class="demo-explainer">
    <p class="hand">a note from the demo</p>
    <p>
      This frontend keeps one current room in your browser. Making and funding a new room replaces
      it. Shared accounts, room history, and real transactions will arrive with the backend.
    </p>
  </div>
</main>
<Footer />
