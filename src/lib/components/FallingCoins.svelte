<script lang="ts">
  import Art from '#lib/components/Art.svelte';

  // Twice the previous size (~110px–160px vs previous ~52px–78px)
  // Dropping randomly across the screen as soon as website loads, then disappearing.
  // Footer is kept as is.
  const coins = [
    { id: 1, left: '4%', size: 136, rotateStart: -24, rotateEnd: 32, delay: 0.05, duration: 1.45, drift: 28 },
    { id: 2, left: '16%', size: 114, rotateStart: 18, rotateEnd: -45, delay: 0.28, duration: 1.6, drift: -24 },
    { id: 3, left: '28%', size: 156, rotateStart: -35, rotateEnd: 40, delay: 0.12, duration: 1.4, drift: 18 },
    { id: 4, left: '40%', size: 122, rotateStart: 12, rotateEnd: -28, delay: 0.42, duration: 1.7, drift: -32 },
    { id: 5, left: '52%', size: 164, rotateStart: -16, rotateEnd: 55, delay: 0.18, duration: 1.5, drift: 22 },
    { id: 6, left: '64%', size: 118, rotateStart: 25, rotateEnd: -36, delay: 0.36, duration: 1.65, drift: -18 },
    { id: 7, left: '76%', size: 148, rotateStart: -22, rotateEnd: 48, delay: 0.08, duration: 1.42, drift: 30 },
    { id: 8, left: '88%', size: 126, rotateStart: 30, rotateEnd: -50, delay: 0.24, duration: 1.58, drift: -25 },
    { id: 9, left: '22%', size: 140, rotateStart: -14, rotateEnd: 28, delay: 0.5, duration: 1.55, drift: 15 },
    { id: 10, left: '70%', size: 130, rotateStart: 20, rotateEnd: -30, delay: 0.45, duration: 1.62, drift: -20 },
  ];
</script>

<!-- Falling coins layer: drops randomly across the screen on load and disappears -->
<div class="falling-coins-layer" aria-hidden="true">
  {#each coins as coin (coin.id)}
    <div
      class="falling-coin"
      style="
        left: {coin.left};
        width: {coin.size}px;
        height: {coin.size}px;
        --rot-start: {coin.rotateStart}deg;
        --rot-end: {coin.rotateEnd}deg;
        --drift-x: {coin.drift}px;
        --fall-delay: {coin.delay}s;
        --fall-duration: {coin.duration}s;
      "
    >
      <Art kind="coin" />
    </div>
  {/each}
</div>

<style>
  .falling-coins-layer {
    position: fixed;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    z-index: 20;
  }

  .falling-coin {
    position: absolute;
    top: 0;
    opacity: 0;
    transform: translate3d(0, -180px, 0) rotate(var(--rot-start));
    will-change: transform, opacity;
    animation: naturalDrop var(--fall-duration) cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards;
    animation-delay: var(--fall-delay);
    filter: drop-shadow(0 14px 28px rgba(0, 0, 0, 0.16));
  }

  @keyframes naturalDrop {
    0% {
      opacity: 0;
      transform: translate3d(0, -180px, 0) rotate(var(--rot-start));
    }
    10% {
      opacity: 1;
    }
    75% {
      opacity: 1;
    }
    90% {
      opacity: 0.8;
    }
    100% {
      /* Dropping fully past viewport with realistic horizontal air drift and rotation, then disappearing */
      opacity: 0;
      transform: translate3d(var(--drift-x), calc(100vh + 190px), 0) rotate(var(--rot-end));
    }
  }

  @media (max-width: 640px) {
    .falling-coin:nth-child(even) {
      display: none;
    }
    .falling-coin {
      transform: scale(0.75) translate3d(0, -180px, 0) rotate(var(--rot-start));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .falling-coins-layer {
      display: none !important;
    }
  }
</style>
