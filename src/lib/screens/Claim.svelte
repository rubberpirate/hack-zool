<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { untrack } from 'svelte';
  import { app, redeem, copyText } from '#lib/state.svelte.ts';
  import { claimAmount, daysLeft } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import QR from '#lib/components/QR.svelte';
  import { enter } from '#lib/motion.ts';
  const token = $derived(page.url.pathname.split('/')[2]);
  const sample = $derived(token === 'demo' || token.startsWith('sample-'));
  const room = $derived(sample ? app.sample : app.room);
  const stage = $derived(
    page.url.pathname.endsWith('/done') ? 3 : page.url.pathname.endsWith('/payout') ? 2 : 1
  );
  let claimNo = $state(4);
  let mode = $state<'wallet' | 'gift'>(
    untrack(() =>
      page.url.searchParams.get('method') === 'gift' && room.giftCards ? 'gift' : 'wallet'
    )
  );
  let address = $state('');
  let brand = $state('Amazon');
  let error = $state('');
  let busy = $state(false);
  let confirmed = $state(false);
  let scanner: HTMLDialogElement;
  $effect(() => {
    const id = token;
    const ready = app.ready;
    if (ready)
      untrack(() => {
        claimNo =
          id === 'demo'
            ? Array.from({ length: room.count }, (_, i) => i + 1).find(
                (n) => !room.claimed.includes(n)
              ) || 1
            : Number(id.split('-').at(-1));
      });
  });
  $effect(() => {
    if (app.ready && !room.giftCards && mode === 'gift') mode = 'wallet';
  });
  const valid = $derived(
    token === 'demo' ||
      (token.startsWith(room.id + '-') &&
        Number.isInteger(claimNo) &&
        claimNo >= 1 &&
        claimNo <= room.count)
  );
  const amount = $derived(claimAmount(room, claimNo));
  const used = $derived(room.claimed.includes(claimNo));
  const gift = $derived(room.gifts.includes(claimNo));
  const blocked = $derived(
    !valid ||
      !room.funded ||
      daysLeft(room) === 0 ||
      room.closed ||
      (used && stage !== 3 && !confirmed)
  );
  async function sweep(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    if (mode === 'wallet' && !/^(u1|zs)[a-z0-9]{28,}$/i.test(address.trim())) {
      error = 'Use a shielded address starting with u1 or zs. Or try the demo scanner.';
      document.getElementById('wallet-address')?.focus();
      return;
    }
    busy = true;
    await new Promise((r) => setTimeout(r, 950));
    if (!redeem(claimNo, mode === 'gift', sample)) {
      error = 'This link has already been used or the room is closed.';
      busy = false;
      return;
    }
    confirmed = true;
    app.giftBrand = brand;
    address = '';
    await goto(`/claim/${room.id}-${claimNo}/done`);
    busy = false;
  }
  function demoScan() {
    address = 'u1demozsplitshieldedwalletaddress00000000000000000000000000000000';
    scanner.close();
    error = '';
  }
</script>

<svelte:head
  ><title
    >{blocked
      ? 'Claim unavailable'
      : stage === 1
        ? "You've got ZEC"
        : stage === 2
          ? 'Pick a payout'
          : 'All yours'} — Z-Split</title
  ></svelte:head
>
<main id="main" class="claim-page" class:claim-done={stage === 3}>
  {#if !app.ready}<div class="claim-loading hand" role="status">Opening your one-time link…</div>
  {:else if blocked || (stage === 3 && !used)}
    <section class="claim-unavailable" use:enter>
      <Art kind="envelope" />
      <p class="hand">
        {!valid
          ? 'this one wandered off.'
          : room.closed || daysLeft(room) === 0
            ? 'the room has wrapped up.'
            : 'one-time means one time.'}
      </p>
      <h1>
        {!valid
          ? 'LINK NOT FOUND.'
          : !room.funded
            ? 'POOL NOT FUNDED.'
            : room.closed
              ? 'ROOM CLOSED.'
              : daysLeft(room) === 0
                ? 'LINK EXPIRED.'
                : 'THIS LINK IS SPENT.'}
      </h1>
      <p class="hand">
        {!valid
          ? 'Check the link you were given, or ask your organizer for a new QR code.'
          : !room.funded
            ? 'Your organizer still needs to fund this pool. Check back in a moment.'
            : room.closed || daysLeft(room) === 0
              ? 'Unclaimed funds have been returned to the organizer’s pool.'
              : 'Someone already claimed this one. Ask your organizer for another link.'}
      </p>
      <Action href="/">Back to the beginning</Action>
    </section>
  {:else if stage === 1}
    <section class="claim-intro" use:enter>
      <div class="claim-hero-art" use:enter={'art'}>
        <Art kind="envelope" />
        <div class="one-time-badge">
          <svg viewBox="0 0 160 160" aria-hidden="true"
            ><circle
              cx="80"
              cy="80"
              r="74"
              fill="var(--lime)"
              stroke="black"
              stroke-width="3"
            /><path id="one-time-ring" d="M80 21a59 59 0 1 1-.1 0" fill="none" /><text
              font-family="sans-serif"
              font-size="13"
              font-weight="800"
              letter-spacing="1"
              ><textPath href="#one-time-ring">ONE-TIME LINK • ONE-TIME LINK • </textPath></text
            ><circle cx="80" cy="80" r="23" fill="white" stroke="black" stroke-width="3" /><path
              d="m73 72-2 6m15-3-2 6m-15 3q8 12 19 0"
              fill="none"
              stroke="black"
              stroke-width="3"
              stroke-linecap="round"
            /></svg
          >
        </div>
      </div>
      <p class="hand claim-kicker">someone just paid you!</p>
      <h1>
        YOU'VE GOT<br /><mark class:long-amount={amount >= 10}>{amount.toFixed(2)} ZEC.</mark>
      </h1>
      <p class="hand claim-description">
        From the {room.name} room. This link works once, so keep your screen to yourself.
      </p>
    </section>
    <div class="claim-bottom">
      <Action href={`/claim/${token}/payout`}>Claim it</Action>
      <div class="recipient-foot">
        <p class="hand muted">no account. no address handed over.</p>
        <div class="progress-dots" aria-label="Step 1 of 3">
          <span class="filled"></span><span></span><span></span>
        </div>
      </div>
    </div>
  {:else if stage === 2}
    <section class="payout-content" use:enter>
      <p class="hand claim-kicker">where should it land?</p>
      <h1>PICK A<br />PAYOUT.</h1>
      <form id="payout-form" onsubmit={sweep}>
        <div class="payout-option" class:active={mode === 'wallet'}>
          <button
            type="button"
            class="payout-option-heading"
            aria-pressed={mode === 'wallet'}
            onclick={() => {
              mode = 'wallet';
              error = '';
            }}
            ><Art kind="shield" /><span
              ><strong>MY SHIELDED WALLET</strong><span class="hand"
                >swept straight in, nothing in between</span
              ></span
            ></button
          >{#if mode === 'wallet'}<div class="wallet-field">
              <label for="wallet-address">YOUR SHIELDED ADDRESS</label>
              <div class="scan-input">
                <input
                  id="wallet-address"
                  placeholder="paste it here"
                  bind:value={address}
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  aria-invalid={!!error}
                  aria-describedby={error ? 'payout-error' : 'address-privacy'}
                /><button type="button" class="pill" onclick={() => scanner.showModal()}
                  >SCAN</button
                >
              </div>
            </div>{/if}
        </div>
        {#if room.giftCards}<div class="payout-option" class:active={mode === 'gift'}>
            <button
              type="button"
              class="payout-option-heading"
              aria-pressed={mode === 'gift'}
              onclick={() => {
                mode = 'gift';
                error = '';
              }}
              ><Art kind="gift" /><span
                ><strong>A GIFT CARD</strong><span class="hand">no wallet? cash out instead</span
                ></span
              ></button
            >{#if mode === 'gift'}<div class="gift-brands">
                <label for="gift-brand">PICK YOUR LITTLE TREAT</label><select
                  id="gift-brand"
                  bind:value={brand}
                  ><option>Amazon</option><option>Steam</option><option>Starbucks</option></select
                >
                <p class="hand">A demo {brand} gift card, paid for with {amount.toFixed(2)} ZEC.</p>
              </div>{/if}
          </div>{/if}
        {#if error}<p id="payout-error" class="form-error hand" role="alert">{error}</p>{/if}
      </form>
    </section>
    <div class="claim-bottom payout-bottom">
      <p id="address-privacy" class="hand muted payout-privacy">
        {mode === 'wallet'
          ? 'this address only goes to the network. the organizer never sees it.'
          : 'just a gift card. no wallet needed. no personal details saved.'}
      </p>
      <button type="submit" form="payout-form" class="action primary has-cap" disabled={busy}
        ><span class="action-label"
          >{busy
            ? 'Sending your payout…'
            : mode === 'wallet'
              ? `Sweep ${amount.toFixed(2)} ZEC`
              : `Get ${brand} gift card`}</span
        ><span class="action-cap"><Icon name="plus" size={26} /></span></button
      >
      <div class="recipient-foot">
        <span></span>
        <div class="progress-dots" aria-label="Step 2 of 3">
          <span class="filled"></span><span class="filled"></span><span></span>
        </div>
      </div>
    </div>
  {:else}
    <section class="done-content" use:enter>
      <div class="success-art" use:enter={'art'}><Art kind="success" /></div>
      <p class="hand claim-kicker">link burned. money moving.</p>
      <h1><mark>ALL</mark><br />YOURS.</h1>
      <p class="hand done-description">
        {amount.toFixed(2)} ZEC is on its way to {gift
          ? 'your gift card.'
          : 'your shielded wallet.'}
      </p>
      <ul class="done-checklist hand">
        <li><span class="check-mini"><Icon name="check" size={20} /></span>Claim link used up</li>
        <li>
          <span class="check-mini"><Icon name="check" size={20} /></span>{gift
            ? 'Demo gift card ready'
            : 'Sweep sent to the network'}
        </li>
        <li>
          <span class="check-mini" class:pending={!gift}
            >{#if gift}<Icon name="check" size={20} />{/if}</span
          >{gift ? 'Ready for a little treat' : 'Confirming in your wallet'}
        </li>
      </ul>
      {#if gift}<button
          class="gift-code"
          onclick={() =>
            copyText(
              `ZSPLIT-DEMO-${String(claimNo).padStart(3, '0')}`,
              'Demo gift card code copied.'
            )}
          ><span class="label">YOUR DEMO GIFT CARD</span><strong
            >ZSPLIT-DEMO-{String(claimNo).padStart(3, '0')}</strong
          ><Icon name="copy" size={20} /></button
        >{/if}
    </section>
    <div class="claim-bottom">
      <Action href="/" icon="close">Close this page</Action>
      <div class="recipient-foot">
        <p class="hand muted">nothing about you is saved here.</p>
        <div class="progress-dots" aria-label="Step 3 of 3">
          <span class="filled"></span><span class="filled"></span><span class="filled"></span>
        </div>
      </div>
    </div>
  {/if}
</main>
<dialog bind:this={scanner} class="scan-dialog">
  <form method="dialog">
    <button class="dialog-x circle-button" aria-label="Close scanner"><Icon name="close" /></button>
    <p class="hand">point, scan, get paid.</p>
    <h2>SCAN YOUR WALLET.</h2>
    <div class="scanner-preview">
      <div class="scan-corner tl"></div>
      <div class="scan-corner tr"></div>
      <div class="scan-corner bl"></div>
      <div class="scan-corner br"></div>
      <QR value="u1demozsplitshieldedwalletaddress00000000000000000000000000000000" />
    </div>
    <p class="hand">This is a demo scanner. Try a sample shielded address to finish the flow.</p>
    <Action onclick={demoScan}>Use demo address</Action>
  </form>
</dialog>
