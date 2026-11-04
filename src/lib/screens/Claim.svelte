<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { app, redeem, copyText, notify } from '#lib/state.svelte.ts';
  import {
    getBackendClaim,
    redeemBackendClaim,
    reportBackendSweep,
    type ClaimMetadata,
  } from '#lib/api.ts';
  import { claimAmount, daysLeft } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import QR from '#lib/components/QR.svelte';
  import { enter } from '#lib/motion.ts';

  const token = $derived(page.url.pathname.split('/')[2]);
  const isBackendClaim = $derived(
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(token)
  );
  const sample = $derived(token === 'demo' || token.startsWith('sample-'));
  const room = $derived(sample ? app.sample : app.room);
  const stage = $derived(
    page.url.pathname.endsWith('/done') ? 3 : page.url.pathname.endsWith('/payout') ? 2 : 1
  );

  let claimNo = $state(4);
  let bearerToken = $state('');
  let backendClaim = $state<ClaimMetadata | null>(null);
  let payoutTxid = $state('');
  let sweptReported = $state(false);

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

  onMount(async () => {
    // Extract bearer token from URL fragment (#t=...)
    if (typeof window !== 'undefined' && location.hash) {
      const match = location.hash.match(/[#&]t=([^&]+)/);
      if (match) bearerToken = match[1];
    }
    // If it's a real backend claim, fetch its metadata
    if (isBackendClaim) {
      try {
        const metadata = await getBackendClaim(token);
        backendClaim = metadata;
      } catch (e) {
        console.warn('Could not fetch claim metadata from backend:', e);
      }
    }
  });

  $effect(() => {
    const id = token;
    const ready = app.ready;
    if (ready && !isBackendClaim)
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
    isBackendClaim ||
      token === 'demo' ||
      (token.startsWith(room.id + '-') &&
        Number.isInteger(claimNo) &&
        claimNo >= 1 &&
        claimNo <= room.count)
  );

  const amount = $derived(
    backendClaim
      ? backendClaim.amount_zat / 100_000_000
      : claimAmount(room, claimNo)
  );

  const used = $derived(
    backendClaim
      ? backendClaim.status !== 'reserved'
      : room.claimed.includes(claimNo)
  );

  const gift = $derived(room.gifts.includes(claimNo));

  const blocked = $derived(
    !valid ||
      (!isBackendClaim && !room.funded) ||
      (!isBackendClaim && daysLeft(room) === 0) ||
      (!isBackendClaim && room.closed) ||
      (used && stage !== 3 && !confirmed)
  );

  async function sweep(event: SubmitEvent) {
    event.preventDefault();
    error = '';

    // Validate shielded address (utest1 unified address, ztestsapling, u1, zs)
    if (
      mode === 'wallet' &&
      !/^(utest1|ztestsapling1|u1|zs)[a-z0-9]{28,}$/i.test(address.trim())
    ) {
      error = 'Enter a valid shielded address (e.g. from Zingo wallet, starting with utest1 or zs).';
      document.getElementById('wallet-address')?.focus();
      return;
    }

    busy = true;

    if (isBackendClaim) {
      // Execute genuine backend redemption
      try {
        if (!bearerToken) {
          throw new Error('Claim bearer token missing from URL fragment (#t=...).');
        }
        const res = await redeemBackendClaim(token, bearerToken, address.trim());
        confirmed = true;
        payoutTxid = res.txid || '';
        await goto(`/claim/${token}/done`);
      } catch (err: any) {
        error = err.message || 'Redemption failed. Check your token or address.';
      } finally {
        busy = false;
      }
      return;
    }

    // Fallback local demo mode
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
    // Fill sample testnet Orchard address compatible with backend validation
    address = 'utest1tapt80r3qx2e2dz9gq35dldu0yvxvquerz9tm6mu20tq4mdsqt0k9vuqahnk5tcu4lr04xsdfmyw0xlsn293mvfa7jk4hmc9luj6gd6s';
    scanner.close();
    error = '';
  }

  async function handleSweepReport() {
    if (isBackendClaim) {
      try {
        await reportBackendSweep(token);
        sweptReported = true;
        notify('Sweep completion recorded!');
      } catch (e) {
        console.error(e);
      }
    }
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
          : 'All yours'} — Zool</title
  ></svelte:head
>
<main id="main" class="claim-page" class:claim-done={stage === 3}>
  {#if !app.ready}<div class="claim-loading hand" role="status">Opening your one-time link…</div>
  {:else if blocked || (stage === 3 && !used && !confirmed)}
    <section class="claim-unavailable" use:enter>
      <Art kind="envelope" />
      <p class="hand">
        {!valid
          ? 'this one wandered off.'
          : !isBackendClaim && (room.closed || daysLeft(room) === 0)
            ? 'the room has wrapped up.'
            : 'one-time means one time.'}
      </p>
      <h1>
        {!valid
          ? 'LINK NOT FOUND.'
          : !isBackendClaim && !room.funded
            ? 'POOL NOT FUNDED.'
            : !isBackendClaim && room.closed
              ? 'ROOM CLOSED.'
              : !isBackendClaim && daysLeft(room) === 0
                ? 'LINK EXPIRED.'
                : 'THIS LINK IS SPENT.'}
      </h1>
      <p class="hand">
        {!valid
          ? 'Check the link you were given, or ask your organizer for a new QR code.'
          : !isBackendClaim && !room.funded
            ? 'Your organizer still needs to fund this pool. Check back in a moment.'
            : !isBackendClaim && (room.closed || daysLeft(room) === 0)
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
        YOU'VE GOT<br /><mark class:long-amount={amount >= 10}>{amount.toFixed(4)} ZEC.</mark>
      </h1>
      <p class="hand claim-description">
        {isBackendClaim ? 'Private shielded payment from Zool.' : `From the ${room.name} room.`} This link works once, so keep your screen to yourself.
      </p>
    </section>
    <div class="claim-bottom">
      <Action href={`/claim/${token}/payout${bearerToken ? `#t=${bearerToken}` : ''}`}>Claim it</Action>
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
              ><strong>MY ZINGO / SHIELDED WALLET</strong><span class="hand"
                >swept straight in, nothing in between</span
              ></span
            ></button
          >{#if mode === 'wallet'}<div class="wallet-field">
              <label for="wallet-address">YOUR SHIELDED RECEIVING ADDRESS (TESTNET)</label>
              <div class="scan-input">
                <input
                  id="wallet-address"
                  placeholder="paste utest1... address from Zingo"
                  bind:value={address}
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  aria-invalid={!!error}
                  aria-describedby={error ? 'payout-error' : 'address-privacy'}
                /><button type="button" class="pill" onclick={() => scanner.showModal()}
                  >SAMPLE</button
                >
              </div>
            </div>{/if}
        </div>
        {#if !isBackendClaim && room.giftCards}<div class="payout-option" class:active={mode === 'gift'}>
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
                <p class="hand">A demo {brand} gift card, paid for with {amount.toFixed(4)} ZEC.</p>
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
            ? 'Broadcasting payout…'
            : mode === 'wallet'
              ? `Sweep ${amount.toFixed(4)} ZEC`
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
      <p class="hand claim-kicker">link burned. shielded payment dispatched.</p>
      <h1><mark>ALL</mark><br />YOURS.</h1>
      <p class="hand done-description">
        {amount.toFixed(4)} ZEC is on its way to {gift
          ? 'your gift card.'
          : 'your shielded wallet.'}
      </p>
      <ul class="done-checklist hand">
        <li><span class="check-mini"><Icon name="check" size={20} /></span>Claim link verified & redeemed</li>
        <li>
          <span class="check-mini"><Icon name="check" size={20} /></span>{gift
            ? 'Demo gift card ready'
            : 'Shielded transaction dispatched by Zool worker'}
        </li>
        <li>
          <span class="check-mini" class:pending={!gift}
            >{#if gift}<Icon name="check" size={20} />{/if}</span
          >{gift ? 'Ready for a little treat' : 'Confirming on Zcash Testnet (check Zingo wallet)'}
        </li>
      </ul>

      {#if isBackendClaim}
        <div class="sweep-report-box">
          <p class="hand muted">Once you see funds in your wallet, you can report the sweep complete:</p>
          <button
            type="button"
            class="pill"
            disabled={sweptReported}
            onclick={handleSweepReport}
          >
            {sweptReported ? '✓ Sweep Self-Reported' : 'Mark as Swept to Personal Wallet'}
          </button>
        </div>
      {/if}

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
    <h2>SAMPLE SHIELDED ADDRESS.</h2>
    <div class="scanner-preview">
      <div class="scan-corner tl"></div>
      <div class="scan-corner tr"></div>
      <div class="scan-corner bl"></div>
      <div class="scan-corner br"></div>
      <QR value="utest1tapt80r3qx2e2dz9gq35dldu0yvxvquerz9tm6mu20tq4mdsqt0k9vuqahnk5tcu4lr04xsdfmyw0xlsn293mvfa7jk4hmc9luj6gd6s" />
    </div>
    <p class="hand">Use a valid testnet Orchard address for this demonstration.</p>
    <Action onclick={demoScan}>Use testnet address</Action>
  </form>
</dialog>

<style>
  .sweep-report-box {
    margin-top: 1.5rem;
    padding: 1rem;
    background: #edf2f7;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: center;
  }
</style>
