<script lang="ts">
  import { onMount } from 'svelte';
  import Art from '#lib/components/Art.svelte';

  let visible = $state(false);

  function scrollToTop(e: MouseEvent) {
    e.preventDefault();
    const topEl = document.getElementById('top') || document.body;
    topEl.scrollIntoView({ behavior: 'smooth' });
  }

  onMount(() => {
    const handleScroll = () => {
      visible = window.scrollY > 120;
    };
    window.addEventListener('scroll', handleScroll, { passive: true });
    handleScroll();
    return () => window.removeEventListener('scroll', handleScroll);
  });
</script>

<button
  type="button"
  class="corner-badge-btn"
  class:visible
  onclick={scrollToTop}
  aria-label="Scroll to top of website"
  title="Scroll to top"
>
  <Art kind="badge" />
</button>

<style>
  .corner-badge-btn {
    position: fixed;
    right: 1.5rem;
    bottom: 1.5rem;
    width: 4.8rem;
    height: 4.8rem;
    border: none;
    background: transparent;
    cursor: pointer;
    z-index: 99;
    padding: 0;
    margin: 0;
    border-radius: 50%;
    filter: drop-shadow(0 4px 14px rgba(0, 0, 0, 0.18));
    opacity: 0;
    pointer-events: none;
    transform: translateY(12px) scale(0.92);
    transition:
      opacity 240ms cubic-bezier(0.22, 1, 0.36, 1),
      transform 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .corner-badge-btn.visible {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(0) scale(1);
  }

  .corner-badge-btn:hover {
    transform: translateY(-3px) scale(1.05) rotate(8deg);
  }

  .corner-badge-btn:active {
    transform: scale(0.96);
  }

  .corner-badge-btn:focus-visible {
    outline: 0.1875rem solid var(--accent);
    outline-offset: 0.25rem;
  }

  @media (max-width: 640px) {
    .corner-badge-btn {
      right: 1rem;
      bottom: 1rem;
      width: 4rem;
      height: 4rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .corner-badge-btn {
      transition: opacity 150ms ease !important;
      transform: none !important;
    }
    .corner-badge-btn:hover,
    .corner-badge-btn:active {
      transform: none !important;
    }
  }
</style>
