<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { app, claimLink, copyText, notify } from '#lib/state.svelte.ts';
  import { claimAmount, daysLeft, formatZatToZec, prizeRankLabel } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import QR from '#lib/components/QR.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import { enter } from '#lib/motion.ts';
  const unavailable = $derived(app.room.closed || daysLeft(app.room) === 0);
  const unavailableStatus = $derived(app.room.closed ? 'CLOSED' : 'EXPIRED');
  $effect(() => {
    if (app.ready && !app.room.funded) void goto('/room/fund', { replace: true });
  });
  let page = $state(0);
  let origin = $state('');
  let exporting = $state(false);
  let printing = $state(false);
  let showEmailModal = $state(false);
  let sendingEmails = $state(false);
  let emailApiKey = $state(typeof localStorage !== 'undefined' ? localStorage.getItem('zool-resend-key') || '' : '');
  let emailStatus = $state('');
  let bulkEmailText = $state('');
  let recipientList = $state<string[]>([]);

  $effect(() => {
    if (recipientList.length !== app.room.count) {
      recipientList = Array.from({ length: app.room.count }, (_, i) => app.recipients?.[i]?.email || '');
    }
  });

  function applyBulkEmails() {
    const lines = bulkEmailText
      .split(/[\n,;]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    for (let i = 0; i < app.room.count; i++) {
      recipientList[i] = lines[i] || '';
    }
    notify(`Applied ${lines.length} emails to claims list.`);
  }

  async function sendEmails() {
    const toSend = recipientList
      .map((email, idx) => {
        const number = idx + 1;
        const customRec = app.recipients?.[idx];
        const amountZat = app.issuedTickets?.[idx]?.amount_zat ??
          (customRec?.prize ? Math.round(Number(customRec.prize) * 100_000_000) : Math.round(claimAmount(app.room, number) * 100_000_000));
        return {
          email: email.trim(),
          number,
          amountZec: formatZatToZec(amountZat),
          prizeLabel: customRec?.rank || prizeRankLabel(number, app.room.split),
          claimUrl: claimLink(number),
        };
      })
      .filter((r) => !!r.email);

    if (toSend.length === 0) {
      notify('Enter at least one participant email address.');
      return;
    }

    if (emailApiKey) {
      try { localStorage.setItem('zool-resend-key', emailApiKey); } catch {}
    }

    sendingEmails = true;
    emailStatus = `Sending ${toSend.length} claim email(s) via Resend…`;

    try {
      const res = await fetch('/resend-email', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          apiKey: emailApiKey,
          roomName: app.room.name || 'Zool Payout',
          recipients: toSend,
        }),
      });

      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'Failed to dispatch emails');

      emailStatus = `✓ Successfully delivered ${data.delivered} of ${toSend.length} emails!`;
      notify(`Sent ${data.delivered} claim email(s) via Resend!`);
    } catch (err: any) {
      emailStatus = `Error: ${err.message || 'Failed to send'}`;
      notify(`Email failed: ${err.message || 'Error'}`);
    } finally {
      sendingEmails = false;
    }
  }
  onMount(() => {
    origin = location.origin;
    const after = () => {
      printing = false;
    };
    window.addEventListener('afterprint', after);
    return () => {
      window.removeEventListener('afterprint', after);
    };
  });
  async function printSheet() {
    try {
      await import('qrcode');
      printing = true;
      await tick();
      // Let the newly mounted QR cards finish their cached-module microtasks.
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      window.print();
    } catch {
      notify('Could not prepare printing. Try Download PDF instead.');
    } finally {
      printing = false;
    }
  }

  const pages = $derived(Math.ceil(app.room.count / 12));
  const numbers = $derived(
    Array.from(
      { length: printing ? app.room.count : Math.min(12, app.room.count - page * 12) },
      (_, i) => (printing ? 0 : page * 12) + i + 1
    )
  );
  async function pdf() {
    exporting = true;
    try {
      const { downloadClaimsPDF } = await import('#lib/pdf.ts');
      downloadClaimsPDF(app.room, location.origin);
      notify('Your claim sheet is downloaded. Hand it out with care.');
    } catch {
      notify('Could not export the sheet. Try Print sheet instead.');
    } finally {
      exporting = false;
    }
  }
  function copyAll() {
    if (unavailable) {
      notify('This room is closed or expired. Start a new room to share claims.');
      return;
    }
    const links = Array.from({ length: app.room.count }, (_, i) => i + 1)
      .filter((n) => !app.room.claimed.includes(n))
      .map((n) => {
        const amt = formatZatToZec(
          app.issuedTickets?.[n - 1]?.amount_zat ??
          Math.round(claimAmount(app.room, n) * 100_000_000)
        );
        const rank = prizeRankLabel(n, app.room.split);
        return `${String(n).padStart(2, '0')} (${rank}) — ${amt} TAZ — ${claimLink(n)}`;
      })
      .join('\n');
    if (!links) {
      notify('All links have been claimed. Nothing left to copy.');
      return;
    }
    copyText(links, 'All open claim links copied. Keep them private.');
  }
  let showDispatch = $state(false);
  let dispatched = $state(false);
  function sendToWinners() {
    dispatched = true;
    notify('Unique claim links & QR codes sent to winner emails!');
    setTimeout(() => {
      showDispatch = false;
    }, 1800);
  }
</script>

<svelte:head><title>Your claim links — Zool</title></svelte:head>
<main id="main" class="workspace claims-page" use:enter>
  <p class="hand intro">
    {unavailable
      ? 'this room has wrapped up. these links are no longer active.'
      : 'pool funded. now hand them out or email directly to winners.'}
  </p>
  <h1>{app.room.count} CLAIM LINKS, READY.</h1>
  <div class="sheet-actions">
    <button
      class="pill primary-pill"
      style="background: #111; color: #fff; font-weight: 900;"
      onclick={() => (showEmailModal = true)}
    >
      📧 SEND TO RECIPIENTS (RESEND)
    </button>
    <button class="pill" onclick={printSheet}>PRINT SHEET</button><button
      class="pill"
      disabled={exporting}
      onclick={pdf}>{exporting ? 'PREPARING…' : 'DOWNLOAD PDF'}</button
    ><button class="pill" onclick={copyAll}>COPY ALL LINKS</button>
    <Action href="/room/live">Open live room</Action>
  </div>

  {#key page}<div class="qr-grid" use:enter={'cards'}>
      {#each numbers as number}<a
          class="qr-card"
          class:claimed={app.room.claimed.includes(number)}
          class:closed={unavailable}
          href={claimLink(number)}
          aria-label={`Claim ${number}, ${prizeRankLabel(number, app.room.split)}, ${formatZatToZec(app.issuedTickets?.[number - 1]?.amount_zat ?? Math.round(claimAmount(app.room, number) * 100_000_000))} TAZ, ${app.room.claimed.includes(number) ? 'claimed' : unavailable ? unavailableStatus.toLowerCase() : 'open'}`}
          ><div class="qr-card-heading">
            <h2>No. {String(number).padStart(2, '0')}</h2>
            <span class="status-chip"
              >{app.room.claimed.includes(number)
                ? 'CLAIMED'
                : unavailable
                  ? unavailableStatus
                  : 'OPEN'}</span
            >
          </div>
          <div style="font-size: 0.72rem; font-weight: 700; color: #4a5568; margin-top: -0.4rem; margin-bottom: 0.4rem;">
            {app.recipients?.[number - 1]?.rank || prizeRankLabel(number, app.room.split)}
          </div>
          {#if app.room.claimed.includes(number)}
            <div class="claimed-art">
              <Art kind="coin-check" />
            </div>
          {:else if unavailable}<div class="claimed-art closed-art">
              <span><Icon name="close" size={48} /></span>
            </div>{:else if origin}<QR
              value={claimLink(number)}
            />{:else}<div class="qr-image"></div>{/if}
          <div class="qr-card-foot hand">
            <span>
              {formatZatToZec(
                app.issuedTickets?.[number - 1]?.amount_zat ??
                (app.recipients?.[number - 1]?.prize ? Math.round(Number(app.recipients[number - 1].prize) * 100_000_000) : Math.round(claimAmount(app.room, number) * 100_000_000))
              )}<br />TAZ
            </span>
            <span class="muted"
              >{app.room.claimed.includes(number)
                ? 'link is'
                : unavailable
                  ? 'room is'
                  : 'works'}<br />{app.room.claimed.includes(number)
                ? 'dead'
                : unavailable
                  ? unavailableStatus.toLowerCase()
                  : 'once'}</span
            >
          </div></a
        >{/each}
    </div>{/key}
  <div class="sheet-footer">
    <nav class="pagination" aria-label="Claim sheet pages">
      <button
        class="circle-button"
        aria-label="Previous page"
        disabled={page === 0}
        onclick={() => page--}><Icon name="left" /></button
      ><span
        >{page * 12 + 1}–{Math.min((page + 1) * 12, app.room.count)} OF<br />{app.room.count}</span
      ><button
        class="circle-button"
        aria-label="Next page"
        disabled={page >= pages - 1}
        onclick={() => page++}><Icon name="right" /></button
      >
    </nav>
    <p class="hand">a claimed QR goes dead, so a photo of it is useless.</p>
  </div>

  {#if showEmailModal}
    <div
      class="modal-backdrop"
      style="position: fixed; inset: 0; background: rgba(0,0,0,0.65); display: flex; align-items: center; justify-content: center; z-index: 9999; padding: 1rem;"
      onclick={() => (showEmailModal = false)}
      role="presentation"
    >
      <div
        class="modal-card"
        style="background: #ffffff; color: #111; max-width: 620px; width: 100%; border: 2px solid #000; border-radius: 12px; padding: 1.5rem; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.2);"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => { if (e.key === 'Escape') showEmailModal = false; }}
        tabindex="-1"
        role="dialog"
        aria-modal="true"
      >
        <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 2px dashed #cbd5e0; padding-bottom: 0.75rem; margin-bottom: 0.75rem;">
          <h2 style="margin: 0; font-size: 1.25rem; font-weight: 900;">📧 Distribute Claims via Resend</h2>
          <button class="circle-button" onclick={() => (showEmailModal = false)}>✕</button>
        </div>
        <p class="hand" style="font-size: 0.85rem; color: #4a5568; margin: 0 0 1rem;">
          Send individual claim links to each participant at once. Each recipient receives only their designated share (1st, 2nd, 3rd place prizes).
        </p>

        <div style="margin-bottom: 1rem;">
          <label for="bulk-emails" style="font-size: 0.75rem; font-weight: 800; display: block; margin-bottom: 0.25rem;">BULK PASTE EMAILS (ONE PER LINE OR COMMA-SEPARATED)</label>
          <textarea
            id="bulk-emails"
            rows="3"
            placeholder="alice@example.com&#10;bob@example.com&#10;charlie@example.com"
            bind:value={bulkEmailText}
            style="width: 100%; font-family: monospace; font-size: 0.8rem; padding: 0.5rem; border: 1px solid #cbd5e0; border-radius: 6px; box-sizing: border-box;"
          ></textarea>
          <button type="button" class="pill" style="margin-top: 0.4rem; font-size: 0.75rem;" onclick={applyBulkEmails}>
            ↓ Apply to Participant Slots
          </button>
        </div>

        <div style="max-height: 240px; overflow-y: auto; border: 1px solid #e2e8f0; border-radius: 8px; padding: 0.75rem; margin-bottom: 1rem; background: #fbfbf9;">
          {#each Array.from({ length: app.room.count }, (_, i) => i + 1) as num}
            {@const customRec = app.recipients?.[num - 1]}
            {@const amountZat = app.issuedTickets?.[num - 1]?.amount_zat ?? (customRec?.prize ? Math.round(Number(customRec.prize) * 100_000_000) : Math.round(claimAmount(app.room, num) * 100_000_000))}
            {@const amt = formatZatToZec(amountZat)}
            {@const rank = customRec?.rank || prizeRankLabel(num, app.room.split)}
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
              <span style="font-size: 0.75rem; font-weight: bold; width: 170px; flex-shrink: 0;">
                No. {String(num).padStart(2, '0')} ({rank}):<br/>
                <span style="color: #2b6cb0;">{amt} TAZ</span>
              </span>
              <input
                type="email"
                placeholder="participant@domain.com"
                bind:value={recipientList[num - 1]}
                style="flex: 1; font-size: 0.8rem; padding: 0.4rem 0.6rem; border: 1px solid #cbd5e0; border-radius: 4px; box-sizing: border-box;"
              />
            </div>
          {/each}
        </div>

        {#if emailStatus}
          <p style="font-size: 0.85rem; font-weight: bold; color: {emailStatus.startsWith('✓') ? '#2ecc71' : '#e53e3e'}; margin-bottom: 0.75rem;">
            {emailStatus}
          </p>
        {/if}

        <div style="display: flex; gap: 0.75rem; justify-content: flex-end;">
          <button type="button" class="pill" onclick={() => (showEmailModal = false)}>Close</button>
          <button
            type="button"
            class="pill"
            style="background: #111; color: #fff;"
            disabled={sendingEmails}
            onclick={sendEmails}
          >
            {sendingEmails ? 'Dispatching Emails…' : '🚀 Send Claim Emails via Resend'}
          </button>
        </div>
      </div>
    </div>
  {/if}
</main>
