<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { app, redeem, copyText, notify } from '#lib/state.svelte.ts';
  import {
    getBackendClaim,
    getBackendClaimStatus,
    redeemBackendClaim,
    reportBackendSweep,
    type ClaimMetadata,
  } from '#lib/api.ts';
  import { claimAmount, daysLeft, formatZatToZec } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import QR from '#lib/components/QR.svelte';
  import { enter } from '#lib/motion.ts';
  import { randomUuid } from '#lib/uuid.ts';

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
  let walletPersisted = $state(false);
  let redemptionKey = '';
  let scanner: HTMLDialogElement;
  let statusChecking = $state(false);
  let statusError = $state('');
  let statusCheckedAt = $state('');
  let refreshStatus = $state<() => void>(() => {});

  const paymentConfirmed = $derived(backendClaim?.status === 'funded_confirmed');
  const redemptionRecorded = $derived(!!backendClaim && !['reserved', 'expired', 'cancelled'].includes(backendClaim.status));
  const transactionBroadcast = $derived(!!backendClaim?.txid && ['funding_broadcast', 'funded_unconfirmed', 'funded_confirmed'].includes(backendClaim.status));
  const progress = $derived(({
    reserved: { title: 'READY TO CLAIM.', text: 'This ticket has not been redeemed yet.', active: false },
    funding_queued: { title: 'PAYOUT QUEUED.', text: 'Your claim is reserved. Waiting for the wallet worker to prepare the payment.', active: true },
    preparing: { title: 'PREPARING PAYOUT.', text: 'Building and proving your shielded transaction. This may take a little while.', active: true },
    funding_broadcast: { title: 'PAYMENT SENT.', text: 'The transaction was broadcast. Waiting for it to be mined on Zcash testnet.', active: true },
    funded_unconfirmed: { title: 'PAYMENT MINED.', text: 'Your transaction is in a block. Waiting for the required confirmations.', active: true },
    funded_confirmed: { title: 'PAYMENT CONFIRMED.', text: 'Funding to your claim wallet is confirmed. You can sweep onward locally in your wallet.', active: false },
    manual_review: { title: 'PAYOUT PAUSED.', text: 'The coordinator must review this payout before it can continue. Contact your organizer; no payment is confirmed yet.', active: false },
    ambiguous_broadcast: { title: 'CHECKING PAYMENT.', text: 'The wallet is checking an uncertain broadcast. Your existing payout remains reserved.', active: true },
    reconciling: { title: 'CHECKING PAYMENT.', text: 'Checking the existing transaction against the chain.', active: true },
    reorged: { title: 'RECHECKING PAYMENT.', text: 'Chain confirmations changed. The wallet is rechecking your existing payment.', active: true },
    expired: { title: 'CLAIM EXPIRED.', text: 'This unused ticket expired. Contact your organizer.', active: false },
    cancelled: { title: 'CLAIM CANCELLED.', text: 'This ticket was cancelled. Contact your organizer.', active: false },
  } as Record<string, { title: string; text: string; active: boolean }>)[backendClaim?.status || ''] || {
    title: statusError ? 'STATUS UNAVAILABLE.' : 'CHECKING STATUS.', text: statusError ? 'Could not reach the latest payout status. Refresh or contact your organizer.' : 'Checking your payout with the coordinator.', active: !statusError,
  });

  onMount(() => {
    // Extract bearer token from URL fragment (#t=...)
    if (typeof window !== 'undefined' && location.hash) {
      const match = location.hash.match(/[#&]t=([^&]+)/);
      if (match) bearerToken = match[1];
    }
    if (!isBackendClaim) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    let failures = 0;
    const check = async () => {
      if (controller.signal.aborted || statusChecking) return;
      clearTimeout(timer);
      statusChecking = true;
      const request = new AbortController();
      const abortRequest = () => request.abort();
      controller.signal.addEventListener('abort', abortRequest, { once: true });
      const timeout = setTimeout(abortRequest, 15_000);
      try {
        if (!bearerToken) throw new Error('The claim token is missing. Open the complete link from your email.');
        const metadata = stage === 3
          ? await getBackendClaimStatus(token, bearerToken, request.signal)
          : await getBackendClaim(token, bearerToken, request.signal);
        if (controller.signal.aborted) return;
        backendClaim = metadata;
        payoutTxid = metadata.txid || '';
        sweptReported = metadata.sweep_reported || false;
        statusError = '';
        statusCheckedAt = new Date().toLocaleTimeString();
        failures = 0;
        if (stage !== 3 && metadata.status !== 'reserved') {
          await goto(`/claim/${token}/done#t=${bearerToken}`, { replace: true });
        }
      } catch (e) {
        if (!controller.signal.aborted) {
          failures++;
          statusError = request.signal.aborted ? 'The status check timed out. Please try again.'
            : e instanceof Error ? e.message : 'Could not refresh payout status.';
        }
      } finally {
        clearTimeout(timeout);
        controller.signal.removeEventListener('abort', abortRequest);
        if (!controller.signal.aborted) {
          statusChecking = false;
          if (bearerToken && !['expired', 'cancelled'].includes(backendClaim?.status || '')) {
            const delay = failures ? Math.min(30_000, 3000 * 2 ** failures)
              : ['funded_confirmed', 'manual_review'].includes(backendClaim?.status || '') ? 15_000 : 3000;
            timer = setTimeout(check, delay);
          }
        }
      }
    };
    refreshStatus = () => { void check(); };
    void check();
    return () => { controller.abort(); clearTimeout(timer); };
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
      (used && !isBackendClaim && stage !== 3 && !confirmed)
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
        if (!walletPersisted) {
          throw new Error('Confirm that this is a separate claim wallet and that its recovery backup is saved.');
        }
        redemptionKey ||= randomUuid();
        const res = await redeemBackendClaim(token, bearerToken, address.trim(), redemptionKey, walletPersisted);
        confirmed = true;
        payoutTxid = res.txid || '';
        await goto(`/claim/${token}/done#t=${bearerToken}`);
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
        await reportBackendSweep(token, bearerToken);
        sweptReported = true;
        notify('Sweep completion recorded!');
      } catch (e) {
        notify(e instanceof Error ? e.message : 'Could not record the sweep report. Try again.');
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
  {:else if blocked || (stage === 3 && !isBackendClaim && !used && !confirmed)}
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
              fill="var(--accent)"
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
              <label for="wallet-address">{isBackendClaim ? 'CLAIM-ONLY SHIELDED RECEIVING ADDRESS (TESTNET)' : 'YOUR SHIELDED RECEIVING ADDRESS (TESTNET)'}</label>
              <div class="scan-input">
                <input
                  id="wallet-address"
                  placeholder={isBackendClaim ? 'utest1... from a separate claim wallet' : 'paste utest1... address from Zingo'}
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
              {#if isBackendClaim}
                <p class="hand muted">Create a separate claim wallet and save its recovery backup. The coordinator receives this address; keep your later personal sweep destination inside your wallet.</p>
                <label class="hand" style="display:flex;gap:0.5rem;align-items:flex-start;margin-top:0.75rem">
                  <input type="checkbox" bind:checked={walletPersisted} />
                  This is a separate claim wallet, and I have saved its recovery backup.
                </label>
              {/if}
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
          ? isBackendClaim ? 'Only the claim wallet receives this payout. Sweep onward locally after confirmation.' : 'this address only goes to the network. the organizer never sees it.'
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
      <div class="success-art coin-check-art" use:enter={'art'}>
        <Art kind={isBackendClaim && !paymentConfirmed ? 'envelope' : 'coin-check'} />
      </div>
      {#if isBackendClaim}
        <h1 class="payment-title">{progress.title}</h1>
        <div class="payment-status-summary" role="status" aria-label="Payout status" aria-live="polite" aria-atomic="true">
          {#if progress.active}
            <svg class="payment-spinner" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2" opacity="0.2" />
              <path d="M12 3a9 9 0 0 1 9 9" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
            </svg>
          {/if}
          <p class="hand">{progress.text}</p>
        </div>
        {#if backendClaim}<p class="hand payment-amount">Amount: {formatZatToZec(backendClaim.amount_zat)} TAZ</p>{/if}
        {#if statusError}<p class="form-error hand" role="alert">Status update failed: {statusError}</p>{/if}
        <div class="payment-refresh">
          <button type="button" class="pill" disabled={statusChecking} onclick={refreshStatus}>
            {statusChecking ? 'Checking status…' : 'Refresh status'}
          </button>
          {#if statusCheckedAt}<span class="hand muted">Last checked {statusCheckedAt}</span>{/if}
        </div>
        <ol class="done-checklist hand" aria-label="Payout progress">
          <li><span class="check-mini" class:pending={!redemptionRecorded}>
            {#if redemptionRecorded}<Icon name="check" size={20} />{/if}
          </span>{redemptionRecorded ? 'Redemption recorded' : backendClaim?.status === 'reserved' ? 'Claim not yet submitted' : backendClaim ? 'Ticket unavailable' : 'Checking claim'}</li>
          <li><span class="check-mini" class:pending={!transactionBroadcast}>
            {#if transactionBroadcast}<Icon name="check" size={20} />{/if}
          </span>{transactionBroadcast ? 'Transaction broadcast' : 'Broadcast not confirmed yet'}</li>
          <li><span class="check-mini" class:pending={!paymentConfirmed}>
            {#if paymentConfirmed}<Icon name="check" size={20} />{/if}
          </span>{paymentConfirmed ? 'Funding confirmed in your claim wallet' : 'Funding not yet confirmed'}</li>
        </ol>
        {#if payoutTxid}
          <div class="payment-transaction">
            <p class="hand">Transaction ID</p><code>{payoutTxid}</code>
            <p class="hand">Confirmations: {backendClaim?.confirmations || 0}</p>
            <button type="button" class="text-link" onclick={() => copyText(payoutTxid, 'Transaction ID copied.')}>Copy transaction ID</button>
          </div>
        {/if}
        {#if backendClaim?.status === 'reserved'}
          <Action href={`/claim/${token}/payout#t=${bearerToken}`}>Continue to claim</Action>
        {/if}
        <div class="sweep-report-box">
          <p class="hand muted">{paymentConfirmed ? 'After you sweep locally in your wallet, you can report completion below. This is a self-report, not chain verification.' : 'Local sweeping is available after funding is confirmed in your claim wallet.'}</p>
          <button type="button" class="pill" disabled={sweptReported || !paymentConfirmed} onclick={handleSweepReport}>
            {sweptReported ? 'Sweep self-reported' : 'Mark as swept locally'}
          </button>
        </div>
      {:else}
        <p class="hand claim-kicker">link burned. z-coin swept with verified tick.</p>
        <h1><mark>ALL</mark><br />YOURS.</h1>
        <p class="hand done-description">{amount.toFixed(4)} ZEC is on its way to {gift ? 'your gift card.' : 'your shielded wallet.'}</p>
        <ul class="done-checklist hand">
          <li><span class="check-mini"><Icon name="check" size={20} /></span>Claim link verified & redeemed</li>
          <li><span class="check-mini"><Icon name="check" size={20} /></span>{gift ? 'Demo gift card ready' : 'Shielded transaction dispatched by Zool worker'}</li>
          <li><span class="check-mini" class:pending={!gift}>{#if gift}<Icon name="check" size={20} />{/if}</span>{gift ? 'Ready for a little treat' : 'Confirming on Zcash Testnet (check Zingo wallet)'}</li>
        </ul>
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


  .done-content h1.payment-title { font-size: clamp(2.5rem, 12vw, 6rem); line-height: 1.05; margin-bottom: 1.5rem; }
  .payment-status-summary { display: flex; align-items: flex-start; gap: 0.75rem; margin-block: 1rem; }
  .payment-status-summary p, .payment-amount { font-size: 1.35rem; line-height: 1.4; }
  .payment-spinner { flex: 0 0 1.5rem; width: 1.5rem; height: 1.5rem; margin-top: 0.2rem; animation: payment-spin 1s linear infinite; }
  .payment-refresh { display: flex; flex-wrap: wrap; align-items: center; gap: 0.75rem; margin-block: 1.25rem; }
  .payment-transaction { margin-block: 1.5rem; min-width: 0; }
  .payment-transaction code { display: block; overflow-wrap: anywhere; margin-block: 0.5rem; }
  @keyframes payment-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .payment-spinner { animation: none; } }
</style>
