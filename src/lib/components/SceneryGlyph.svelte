<script lang="ts">
  import Art from './Art.svelte';
  export type Glyph =
    'coin' | 'shield' | 'gift' | 'cloud' | 'ticket' | 'letter' | 'plane' | 'flower' | 'smile';
  let { kind }: { kind: Glyph } = $props();
</script>

{#if kind === 'coin' || kind === 'shield' || kind === 'gift' || kind === 'cloud'}<Art {kind} />
{:else if kind === 'ticket'}<svg viewBox="0 0 140 170" fill="white" aria-hidden="true"
    ><rect x="5" y="5" width="130" height="160" rx="18" stroke="black" stroke-width="3" /><g
      fill="black"
      ><rect x="21" y="24" width="36" height="36" rx="5" /><rect
        x="81"
        y="24"
        width="36"
        height="36"
        rx="5"
      /><rect x="21" y="82" width="36" height="36" rx="5" /><path
        d="M87 85h22v22h-8V94H87zm-5 39h11v11h-11zm23 13h13v13h-13zm-42-20h8v25h-8zM28 138h20v7H28z"
      /></g
    ></svg
  >
{:else if kind === 'letter'}<svg viewBox="0 0 180 140" aria-hidden="true"
    ><rect
      x="4"
      y="4"
      width="170"
      height="125"
      rx="17"
      fill="white"
      stroke="black"
      stroke-width="3"
    /><path
      d="m5 10 84 67 84-67M5 126l59-63m110 63-61-63"
      fill="none"
      stroke="black"
      stroke-width="3"
    /></svg
  >
{:else if kind === 'plane'}<svg
    viewBox="0 0 170 150"
    fill="white"
    stroke="black"
    stroke-width="3"
    stroke-linejoin="round"
    aria-hidden="true"
    ><path d="m3 61 161-55-46 137-35-50-42 24 3-48zm41 8L164 6 83 93m-42 24 3-48" /></svg
  >
{:else if kind === 'flower'}<svg
    viewBox="0 0 140 140"
    fill="white"
    stroke="black"
    stroke-width="3"
    aria-hidden="true"
    ><path
      d="M68 58C20 6 5 75 49 78 1 97 49 139 67 96c0 50 66 23 35-10 49 16 53-49 9-42 12-48-53-49-43 14Z"
    /><circle cx="79" cy="75" r="18" fill="var(--yellow)" /></svg
  >
{:else}<svg viewBox="0 0 120 120" aria-hidden="true"
    ><circle cx="60" cy="60" r="56" fill="white" stroke="black" stroke-width="3" /><path
      d="M40 45v12m40-12v12M36 74q24 27 48 0"
      fill="none"
      stroke="black"
      stroke-width="3"
      stroke-linecap="round"
    /></svg
  >{/if}
