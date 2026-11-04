import { animate, stagger } from 'motion';
export function enter(node: HTMLElement, kind: 'page' | 'cards' | 'art' = 'page') {
  if (typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const controls =
    kind === 'cards'
      ? animate(
          Array.from(node.children),
          {
            opacity: [0, 1],
            transform: ['translateY(10px)', 'translateY(0)'],
          },
          { delay: stagger(0.035), duration: 0.35, ease: [0.22, 1, 0.36, 1] }
        )
      : animate(
          node,
          {
            opacity: [0, 1],
            transform: [
              kind === 'art' ? 'translateY(12px) rotate(-1.5deg)' : 'translateY(8px)',
              'translateY(0) rotate(0deg)',
            ],
          },
          { duration: 0.35, ease: [0.22, 1, 0.36, 1] }
        );
  return {
    destroy() {
      controls.stop();
    },
  };
}

/** Content stays readable without JS; reveal a marketing section once it enters view. */
export function reveal(node: HTMLElement) {
  if (typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  let controls: { stop: () => void } | undefined;
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (!entry.isIntersecting) return;
      controls = animate(
        node,
        { opacity: [0.2, 1], transform: ['translateY(18px)', 'translateY(0)'] },
        { duration: 0.45, ease: [0.22, 1, 0.36, 1] }
      );
      observer.disconnect();
    },
    { threshold: 0.12 }
  );
  observer.observe(node);
  return {
    destroy() {
      observer.disconnect();
      controls?.stop();
    },
  };
}
