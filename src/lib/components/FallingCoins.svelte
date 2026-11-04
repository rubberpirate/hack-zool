<script lang="ts">
  import Art from '#lib/components/Art.svelte';

  // Configured coins distributed across the width of the page
  // On load, they visibly tumble & fall across the viewport screen with physics and rotation,
  // and then settle and collect at the bottom of the page near the footer!
  const coins = [
    { id: 1, left: '5%', size: 120, rotateStart: -20, rotateEnd: 24, delay: 0.08, duration: 1.45, drift: 20 },
    { id: 2, left: '16%', size: 104, rotateStart: 18, rotateEnd: -28, delay: 0.28, duration: 1.58, drift: -18 },
    { id: 3, left: '28%', size: 136, rotateStart: -26, rotateEnd: 32, delay: 0.14, duration: 1.4, drift: 16 },
    { id: 4, left: '40%', size: 112, rotateStart: 12, rotateEnd: -22, delay: 0.42, duration: 1.65, drift: -22 },
    { id: 5, left: '52%', size: 142, rotateStart: -16, rotateEnd: 36, delay: 0.18, duration: 1.5, drift: 18 },
    { id: 6, left: '64%', size: 108, rotateStart: 22, rotateEnd: -30, delay: 0.36, duration: 1.6, drift: -15 },
    { id: 7, left: '76%', size: 132, rotateStart: -20, rotateEnd: 38, delay: 0.1, duration: 1.42, drift: 24 },
    { id: 8, left: '88%', size: 116, rotateStart: 26, rotateEnd: -34, delay: 0.24, duration: 1.55, drift: -18 },
    { id: 9, left: '22%', size: 128, rotateStart: -14, rotateEnd: 25, delay: 0.48, duration: 1.55, drift: 12 },
    { id: 10, left: '70%', size: 118, rotateStart: 16, rotateEnd: -24, delay: 0.4, duration: 1.62, drift: -16 },
  ];
</script>

<!-- 1. Falling shower layer: immediately visibly drops across the viewport when website loads -->
<div class="falling-coins-viewport" aria-hidden="true">
  {#each coins as coin (coin.id)}
    <div
      class="shower-coin"
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

<!-- 2. Collected coins at the end of the page near the footer -->
<div class="collected-footer-coins" aria-hidden="true">
  {#each coins as coin (coin.id)}
    <div
      class="footer-coin"
      style="
        left: {coin.left};
        width: {coin.size}px;
        height: {coin.size}px;
        --rot: {coin.rotateEnd}deg;
        --appear-delay: {coin.delay + coin.duration - 0.25}s;
      "
    >
      <Art kind="coin" />
    </div>
  {/each}
</div>

<style>
  /* Viewport shower layer: fixed in viewport so user visibly sees coins fall on website load */
  .falling-coins-viewport {
    position: fixed;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    z-index: 20;
  }

  .shower-coin {
    position: absolute;
    top: 0;
    opacity: 0;
    transform: translate3d(0, -180px, 0) rotate(var(--rot-start));
    will-change: transform, opacity;
    animation: dropThroughScreen var(--fall-duration) cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards;
    animation-delay: var(--fall-delay);
    filter: drop-shadow(0 14px 24px rgba(0, 0, 0, 0.15));
  }

  @keyframes dropThroughScreen {
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
    92% {
      opacity: 0.85;
    }
    100% {
      opacity: 0;
      transform: translate3d(var(--drift-x), calc(100vh + 180px), 0) rotate(var(--rot-end));
    }
  }

  /* Collected coins layer: resting and collected at the bottom near the footer */
  .collected-footer-coins {
    position: absolute;
    bottom: -35px;
    left: 0;
    right: 0;
    height: 120px;
    pointer-events: none;
    overflow: visible;
    z-index: 6;
  }

  .footer-coin {
    position: absolute;
    bottom: 0;
    opacity: 0;
    transform: translateY(30px) rotate(var(--rot));
    will-change: transform, opacity;
    animation: settleAtBottom 450ms cubic-bezier(0.22, 1, 0.36, 1) forwards;
    animation-delay: var(--appear-delay);
    filter: drop-shadow(0 6px 14px rgba(0, 0, 0, 0.12));
  }

  @keyframes settleAtBottom {
    0% {
      opacity: 0;
      transform: translateY(-28px) rotate(calc(var(--rot) - 10deg));
    }
    100% {
      opacity: 0.95;
      transform: translateY(0) rotate(var(--rot));
    }
  }

  @media (max-width: 640px) {
    .shower-coin:nth-child(even),
    .footer-coin:nth-child(even) {
      display: none;
    }
    .shower-coin,
    .footer-coin {
      transform: scale(0.72);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .shower-coin {
      display: none !important;
      animation: none !important;
    }
    .footer-coin {
      opacity: 0.95 !important;
      transform: translateY(0) rotate(var(--rot)) !important;
      animation: none !important;
    }
  }
</style>
