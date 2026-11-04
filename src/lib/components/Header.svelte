<script lang="ts">
  import { page } from '$app/state';
  import { scale, fade } from 'svelte/transition';
  import { app } from '#lib/state.svelte.ts';
  import Icon from './Icon.svelte';
  let open = $state(false);
  const path = $derived(page.url.pathname);
  const stage = $derived(
    path === '/room/new' ? 1 : path === '/room/fund' ? 2 : path === '/room/claims' ? 3 : 0
  );
  $effect(() => {
    void path;
    open = false;
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape' && open) {
      open = false;
      (document.querySelector('.menu-button') as HTMLButtonElement)?.focus();
    }
  }}
/>
<header id="top" class="site-header">
  <div class="brand-group">
    <a class="brand" href="/" aria-label="Z-Split home"
      ><svg viewBox="0 0 60 65" aria-hidden="true"
        ><path
          d="M13 1h35q11 0 11 12v29q0 12-12 12H34L27 65l-7-11h-7Q1 54 1 42V13Q1 1 13 1Z"
          fill="black"
        /><path d="M17 12h19v7L24 38h12v7H15v-7l12-19H17z" fill="white" /><path
          d="M42 12v24m0 5v4"
          stroke="white"
          stroke-width="5"
        /></svg
      ></a
    >
    {#if path.startsWith('/claim/demo') || path.startsWith('/claim/sample-')}<span
        class="sample-claim-badge">SAMPLE<br />CLAIM</span
      >{/if}
    {#if path === '/room/live'}<a href="/room/claims" class="claims-badge"
        ><strong>{app.room.count}</strong><span>CLAIMS</span></a
      >{/if}
  </div>
  {#if stage}<nav class="steps" aria-label="Room setup progress">
      {#each ['room', 'fund', 'share'] as step, i}<a
          href={i === 0
            ? '/room/new'
            : i === 1
              ? '/room/fund'
              : app.room.funded
                ? '/room/claims'
                : '/room/fund'}
          class:current={stage === i + 1}
          class:complete={stage > i + 1}
          aria-current={stage === i + 1 ? 'step' : undefined}>{i + 1} {step}</a
        >{#if i < 2}<svg viewBox="0 0 50 18" aria-hidden="true"
            ><path
              d="m1 12 9-7 9 7 10-7 9 7 10-7"
              stroke="black"
              stroke-width="2"
              stroke-dasharray="2 4"
              fill="none"
            /></svg
          >{/if}{/each}
    </nav>{/if}
  <nav class="desktop-nav" aria-label="Site navigation">
    <a href="/room/new" class="nav-cta" aria-current={path.startsWith('/room/new') ? 'page' : undefined}>
      <Icon name="plus" size={14} /> Pay a room
    </a>
    <a href="/claim" aria-current={path === '/claim' ? 'page' : undefined}>Open a claim</a>
    <a href="/rooms" aria-current={path === '/rooms' ? 'page' : undefined}>Your room</a>
    <a href="/how-it-works" aria-current={path === '/how-it-works' ? 'page' : undefined}>How it works</a>
    <a href="/privacy" aria-current={path === '/privacy' ? 'page' : undefined}>Privacy</a>
  </nav>
  <div class="menu-wrap">
    <button
      class="pill menu-button"
      disabled={!app.ready}
      aria-expanded={open}
      aria-controls="main-menu"
      onclick={() => (open = !open)}
      >{open ? 'CLOSE' : 'MENU'}{#if open}<Icon name="close" size={15} />{/if}</button
    >
    {#if open}<nav
        id="main-menu"
        class="menu-panel"
        aria-label="Main navigation"
        transition:scale={{ duration: 180, start: 0.95, opacity: 0 }}
      >
        <a href="/" class:active={path === '/'}>Home <span>↗</span></a>
        <a href="/room/new" class:active={path === '/room/new'}><strong>Pay a room</strong> <span class="badge-new">NEW</span></a>
        <a href="/claim" class:active={path === '/claim'}>Open a claim <span>↗</span></a>
        <a href="/rooms" class:active={path === '/rooms'}>Your room <span>↗</span></a>
        <a href="/how-it-works" class:active={path === '/how-it-works'}>How it works <span>↗</span></a>
        <a href="/privacy" class:active={path === '/privacy'}>Privacy, plainly <span>↗</span></a>
        <p>A little less admin.<br />A lot more privacy.</p>
        <span class="demo-tag">INTERACTIVE DEMO · NO REAL FUNDS</span>
      </nav>{/if}
  </div>
</header>
{#if open}<button
    tabindex="-1"
    class="menu-dismiss"
    aria-label="Close menu"
    transition:fade={{ duration: 150 }}
    onclick={() => (open = false)}
  ></button>{/if}
