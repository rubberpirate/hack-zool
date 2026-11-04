<script lang="ts">
  let {
    name,
    size = 24,
    stroke = 2.2,
  }: { name: string; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'plus'}<path d="M12 5v14M5 12h14" />
  {:else if name === 'close'}<path d="m6 6 12 12M18 6 6 18" />
  {:else if name === 'check'}<path d="m5 12 5 5L20 7" />
  {:else if name === 'left'}<path d="m12 5-7 7 7 7M5 12h14" />
  {:else if name === 'right'}<path d="m12 5 7 7-7 7M19 12H5" />
  {:else if name === 'copy'}<rect x="8" y="8" width="12" height="13" rx="2" /><path
      d="M16 8V3H3v13h5"
    />
  {:else if name === 'shield'}<path d="m12 2 8 4v7c0 5-8 9-8 9S4 18 4 13V6z" /><path
      d="m8 12 3 3 5-5"
    />
  {:else if name === 'scan'}<path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5M3 12h18" />
  {:else if name === 'download'}<path d="M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4" />
  {/if}
</svg>
