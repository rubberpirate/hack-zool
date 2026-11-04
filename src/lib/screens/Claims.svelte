<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { app, claimLink, copyText, notify } from '#lib/state.svelte.ts';
  import { claimAmount, daysLeft } from '#lib/model.ts';
  import Action from '#lib/components/Action.svelte';
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
      .map(
        (n) =>
          `${String(n).padStart(2, '0')} — ${claimAmount(app.room, n).toFixed(2)} ZEC — ${claimLink(n)}`
      )
      .join('\n');
    if (!links) {
      notify('All links have been claimed. Nothing left to copy.');
      return;
    }
    copyText(links, 'All open claim links copied. Keep them private.');
  }
</script>

<svelte:head><title>Your claim links — Z-Split</title></svelte:head>
<main id="main" class="workspace claims-page" use:enter>
  <p class="hand intro">
    {unavailable
      ? 'this room has wrapped up. these links are no longer active.'
      : 'pool funded. now hand them out.'}
  </p>
  <h1>{app.room.count} CLAIM LINKS, READY.</h1>
  <div class="sheet-actions">
    <button class="pill" onclick={printSheet}>PRINT SHEET</button><button
      class="pill"
      disabled={exporting}
      onclick={pdf}>{exporting ? 'PREPARING…' : 'DOWNLOAD PDF'}</button
    ><button class="pill" onclick={copyAll}>COPY ALL LINKS</button><Action href="/room/live"
      >Open live room</Action
    >
  </div>
  {#key page}<div class="qr-grid" use:enter={'cards'}>
      {#each numbers as number}<a
          class="qr-card"
          class:claimed={app.room.claimed.includes(number)}
          class:closed={unavailable}
          href={`/claim/${app.room.id}-${number}`}
          aria-label={`Claim ${number}, ${claimAmount(app.room, number).toFixed(2)} ZEC, ${app.room.claimed.includes(number) ? 'claimed' : unavailable ? unavailableStatus.toLowerCase() : 'open'}`}
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
          {#if app.room.claimed.includes(number)}<div class="claimed-art">
              <span><Icon name="check" size={55} stroke={2.4} /></span><i></i><i></i>
            </div>{:else if unavailable}<div class="claimed-art closed-art">
              <span><Icon name="close" size={48} /></span>
            </div>{:else if origin}<QR
              value={`${origin}/claim/${app.room.id}-${number}`}
            />{:else}<div class="qr-image"></div>{/if}
          <div class="qr-card-foot hand">
            <span>{claimAmount(app.room, number).toFixed(2)}<br />ZEC</span><span class="muted"
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
</main>
