<script lang="ts">
  import { onMount } from 'svelte';
  let { value, label = 'Scan this one-time claim link' }: { value: string; label?: string } =
    $props();
  let svg = $state('');
  let failed = $state(false);
  onMount(() => {
    let active = true;
    import('qrcode')
      .then(async ({ default: qr }) => {
        const result = await qr.toString(value, {
          type: 'svg',
          margin: 1,
          errorCorrectionLevel: 'M',
        });
        if (active) svg = result;
      })
      .catch(() => (failed = true));
    return () => {
      active = false;
    };
  });
</script>

<div class="qr-image" role="img" aria-label={label}>
  {#if svg}{@html svg}{:else if failed}<span class="qr-fallback">Open claim ↗</span>{:else}<span
      class="qr-placeholder"
    ></span>{/if}
</div>
