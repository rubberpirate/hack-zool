import { animate, stagger } from 'motion';
export function enter(node: HTMLElement, kind: 'page' | 'cards' | 'art' = 'page') {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const controls =
    kind === 'cards'
      ? animate(
          Array.from(node.children),
          {
            opacity: [0, 1],
            transform: ['translateY(14px) rotate(-1deg)', 'translateY(0) rotate(0deg)'],
          },
          { delay: stagger(0.04), type: 'spring', duration: 0.5, bounce: 0.2 }
        )
      : animate(
          node,
          {
            opacity: [0, 1],
            transform: [
              kind === 'art' ? 'translateY(18px) rotate(-3deg)' : 'translateY(10px)',
              'translateY(0) rotate(0deg)',
            ],
          },
          { type: 'spring', duration: 0.5, bounce: 0.2 }
        );
  return {
    destroy() {
      controls.stop();
    },
  };
}

/** Content stays readable without JS; reveal a marketing section once it enters view. */
export function reveal(node: HTMLElement) {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  let controls: { stop: () => void } | undefined;
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (!entry.isIntersecting) return;
      controls = animate(
        node,
        { opacity: [0.25, 1], transform: ['translateY(24px)', 'translateY(0)'] },
        { duration: 0.65, ease: [0.23, 1, 0.32, 1] }
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
