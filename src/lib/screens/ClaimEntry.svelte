<script lang="ts">
  import { goto } from '$app/navigation';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import { enter } from '#lib/motion.ts';
  let link = $state('');
  let error = $state('');
  async function open(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    try {
      const url = new URL(link.trim(), location.origin);
      if (!/^\/claim\/[^/]+\/?$/.test(url.pathname)) throw new Error('invalid');
      await goto(url.pathname.replace(/\/$/, ''));
    } catch {
      error =
        'Paste a complete Z-Split claim link, like /claim/demo. Your organizer can send it again if you need it.';
    }
  }
</script>

<svelte:head><title>Open your claim — Z-Split</title></svelte:head>
<main id="main" class="workspace claim-entry-page" use:enter>
  <div class="claim-entry-copy">
    <p class="hand intro">a little something with your name off it.</p>
    <h1>GOT A LINK?<br /><mark>LET’S OPEN IT.</mark></h1>
    <p>
      Your organizer gave you a one-time claim link or QR code. Paste the link below to see what’s
      waiting for you.
    </p>
    <form onsubmit={open}>
      <label for="claim-link">YOUR CLAIM LINK</label><input
        id="claim-link"
        type="text"
        inputmode="url"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        bind:value={link}
        placeholder="https://z-split…/claim/…"
        aria-invalid={!!error}
        aria-describedby={error ? 'claim-link-error' : 'link-private'}
      />{#if error}<p id="claim-link-error" class="form-error" role="alert">{error}</p>{/if}<Action
        type="submit">Open my claim</Action
      >
    </form>
    <p id="link-private" class="hand entry-privacy">This link works once. Keep it to yourself.</p>
    <div class="entry-demo">
      <span>Just looking around?</span><a href="/claim/demo" class="inline-link"
        >Try a sample claim</a
      >
    </div>
  </div>
  <div class="claim-entry-art">
    <Art kind="envelope" /><span class="hand">no account.<br />just your share.</span>
  </div>
</main>
<Footer />
