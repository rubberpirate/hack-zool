<script lang="ts">
  import { animate } from 'motion';
  import { untrack } from 'svelte';
  let { value }: { value: number } = $props();
  let display = $state(untrack(() => value));
  $effect(() => {
    const end = value;
    if (
      typeof matchMedia === 'undefined' ||
      matchMedia('(prefers-reduced-motion: reduce)').matches
    ) {
      display = end;
      return;
    }
    const animation = animate(
      untrack(() => display),
      end,
      { duration: 0.65, ease: [0.23, 1, 0.32, 1], onUpdate: (n) => (display = Math.round(n)) }
    );
    return () => animation.stop();
  });
</script>

<span class="counter">{display}</span>
