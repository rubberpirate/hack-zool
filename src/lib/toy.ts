import { animate } from 'motion';

/** Keep the resting composition on the parent; only move its interactive inner layer. */
export function toy(node: HTMLButtonElement, onPlay: () => void) {
  const piece = node.querySelector<HTMLElement>('.scene-piece')!;
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const fine = matchMedia('(hover: hover) and (pointer: fine)');
  let controls: { stop: () => void } | undefined;
  let pointer: number | null = null;
  let startX = 0,
    startY = 0,
    x = 0,
    y = 0,
    frame = 0,
    generation = 0;
  let rect: DOMRect;
  function reset() {
    cancelAnimationFrame(frame);
    controls?.stop();
    delete node.dataset.dragging;
    if (reduced.matches) piece.style.transform = '';
    else
      controls = animate(
        piece,
        { transform: 'translate3d(0, 0, 0) rotate(0deg)' },
        { type: 'spring', duration: 0.5, bounce: 0.2 }
      );
  }
  function toss() {
    onPlay();
    if (reduced.matches) return;
    const id = ++generation;
    controls?.stop();
    const flight = animate(
      piece,
      { transform: 'translate3d(0, -42px, 0) rotate(12deg)' },
      { type: 'spring', duration: 0.5, bounce: 0.2 }
    );
    controls = flight;
    void flight.then(() => {
      if (generation === id) reset();
    });
  }
  function down(event: PointerEvent) {
    if (pointer !== null || event.button !== 0) return;
    ++generation;
    controls?.stop();
    pointer = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    x = 0;
    y = 0;
    rect = node.getBoundingClientRect();
    node.setPointerCapture(pointer);
    node.dataset.dragging = 'true';
  }
  function move(event: PointerEvent) {
    if (pointer !== null && event.pointerId !== pointer) return;
    if (pointer === null && (!fine.matches || reduced.matches)) return;
    if (!rect) rect = node.getBoundingClientRect();
    cancelAnimationFrame(frame);
    const dragging = pointer !== null;
    x = dragging
      ? Math.tanh((event.clientX - startX) / 220) * 220
      : (event.clientX - rect.left - rect.width / 2) * 0.07;
    y = dragging
      ? Math.tanh((event.clientY - startY) / 160) * 160
      : (event.clientY - rect.top - rect.height / 2) * 0.07;
    frame = requestAnimationFrame(() => {
      piece.style.transform = `translate3d(${x}px, ${y}px, 0) rotate(${Math.max(-18, Math.min(18, x / 8))}deg)`;
    });
  }
  function up(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    pointer = null;
    if (node.hasPointerCapture(event.pointerId)) node.releasePointerCapture(event.pointerId);
    if (event.type === 'pointerup' && Math.hypot(x, y) < 5) toss();
    else {
      onPlay();
      reset();
    }
    delete node.dataset.dragging;
  }
  function leave() {
    if (pointer === null) reset();
  }
  function click(event: MouseEvent) {
    if (event.detail === 0) toss();
  }
  function enter() {
    rect = node.getBoundingClientRect();
  }
  node.addEventListener('pointerdown', down);
  node.addEventListener('pointermove', move);
  node.addEventListener('pointerup', up);
  node.addEventListener('pointercancel', up);
  node.addEventListener('lostpointercapture', up);
  node.addEventListener('pointerenter', enter);
  node.addEventListener('pointerleave', leave);
  node.addEventListener('click', click);
  return {
    destroy() {
      ++generation;
      cancelAnimationFrame(frame);
      controls?.stop();
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointercancel', up);
      node.removeEventListener('lostpointercapture', up);
      node.removeEventListener('pointerenter', enter);
      node.removeEventListener('pointerleave', leave);
      node.removeEventListener('click', click);
    },
  };
}
