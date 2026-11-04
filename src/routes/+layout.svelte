<script lang="ts">
  import '../app.css';
  import '../refinements.css';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { app, loadRoom } from '#lib/state.svelte.ts';
  import Header from '#lib/components/Header.svelte';
  import Icon from '#lib/components/Icon.svelte';
  let { children } = $props();
  onMount(() => {
    loadRoom();
    const handler = (e: StorageEvent) => {
      if (e.key === 'z-split-room') loadRoom();
    };
    window.addEventListener('storage', handler);
    return () => window.removeEventListener('storage', handler);
  });
  const recipient = $derived(page.url.pathname.startsWith('/claim/'));
</script>

<svelte:head
  ><title>Zool — Pay the room. Collect nothing.</title><meta
    name="description"
    content="Shielded ZEC payouts, hand-delivered. Pay a room with one-time claim links. No wallet lists. No spreadsheets."
  /><link
    rel="preload"
    href="/fonts/anton.woff2"
    as="font"
    type="font/woff2"
    crossorigin="anonymous"
  /><link
    rel="preload"
    href="/fonts/patrick-hand.woff2"
    as="font"
    type="font/woff2"
    crossorigin="anonymous"
  /></svelte:head
>
<a class="skip-link" href="#main">Skip to content</a>
<div class="paper" class:recipient><Header />{@render children()}</div>
<div class="toast" inert={!app.toast} class:visible={!!app.toast} role="status" aria-live="polite">
  <span class="check-mini"><Icon name="check" size={16} /></span>{app.toast}<button
    onclick={() => (app.toast = '')}
    aria-label="Dismiss notification"><Icon name="close" size={18} /></button
  >
</div>
