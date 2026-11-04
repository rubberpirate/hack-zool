<script lang="ts">
  import SceneryGlyph, { type Glyph } from './SceneryGlyph.svelte';
  import { toy } from '#lib/toy.ts';
  import { app } from '#lib/state.svelte.ts';
  let played = $state('');
  const pieces: { kind: Glyph; class: string; label: string }[] = [
    { kind: 'coin', class: 'scene-coin first', label: 'ZEC coin' },
    { kind: 'ticket', class: 'scene-qr first', label: 'Claim ticket' },
    { kind: 'shield', class: 'scene-shield first', label: 'Happy privacy shield' },
    { kind: 'letter', class: 'scene-letter first', label: 'Payout envelope' },
    { kind: 'gift', class: 'scene-gift', label: 'Gift box' },
    { kind: 'plane', class: 'scene-plane', label: 'Paper plane' },
    { kind: 'cloud', class: 'scene-cloud', label: 'Smiling cloud' },
    { kind: 'shield', class: 'scene-shield second', label: 'Little privacy shield' },
    { kind: 'coin', class: 'scene-coin second', label: 'Little ZEC coin' },
    { kind: 'flower', class: 'scene-flower', label: 'Flower' },
    { kind: 'ticket', class: 'scene-qr second', label: 'Little claim ticket' },
    { kind: 'smile', class: 'scene-smile', label: 'Smiley face' },
  ];
</script>

<div
  class="scenery"
  role="group"
  aria-label="Play with the payout doodles. Drag any piece or press Enter to give it a bounce."
>
  {#each pieces as piece}<button
      type="button"
      disabled={!app.ready}
      class="scene-toy {piece.class}"
      aria-label={`Play with ${piece.label.toLowerCase()}`}
      use:toy={() => (played = `${piece.label} bounced back.`)}
      ><span class="scene-piece"><SceneryGlyph kind={piece.kind} /></span></button
    >{/each}
</div>
<span class="sr-only" role="status" aria-live="polite">{played}</span>
