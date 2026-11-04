<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { setAuthToken } from '#lib/api.ts';
  let token = $state('');
  let error = $state('');
  let busy = $state(false);
  let ready = $state(false);
  onMount(() => {
    ready = true;
  });
  async function signIn(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = '';
    try {
      const response = await fetch('/organizer/session', {
        method: 'POST',
        headers: { Authorization: `Bearer ${token.trim()}` },
      });
      if (!response.ok) throw new Error('The organizer token was not accepted.');
      setAuthToken(token.trim());
      token = '';
      await goto('/room/new');
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Could not sign in. Try again.';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head
  ><title>Organizer access — Zool</title><meta name="robots" content="noindex" /></svelte:head
>
<main id="main" style="max-width: 36rem; margin: 4rem auto; padding: 1.5rem">
  <h1>ORGANIZER ACCESS.</h1>
  <p>
    Enter the organizer token supplied by your deployment operator. Recipients only need their claim
    link.
  </p>
  <form onsubmit={signIn}>
    <div class="field">
      <label for="organizer-token">ORGANIZER TOKEN</label>
      <input
        id="organizer-token"
        type="password"
        bind:value={token}
        required
        disabled={!ready}
        autocomplete="current-password"
      />
    </div>
    <p class="muted">This browser remembers your token. Use a trusted device.</p>
    {#if error}<p role="alert">{error}</p>{/if}
    <button class="pill" type="submit" disabled={busy || !ready}
      >{busy ? 'Signing in…' : 'Open organizer tools →'}</button
    >
  </form>
</main>
