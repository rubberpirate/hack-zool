import { demoRoom, validRoom, daysLeft, type Room } from './model';
export const app = $state({
  room: structuredClone(demoRoom),
  sample: {
    ...structuredClone(demoRoom),
    id: 'sample',
    claimed: [] as number[],
    gifts: [] as number[],
    createdAt: Date.now(),
  },
  ready: false,
  toast: '',
  payout: 'wallet' as 'wallet' | 'gift',
  giftBrand: 'Amazon',
  lastClaim: 4,
});
export function loadRoom() {
  try {
    const stored = JSON.parse(localStorage.getItem('z-split-room') || 'null');
    if (validRoom(stored)) app.room = stored;
    const sample = JSON.parse(sessionStorage.getItem('z-split-sample') || 'null');
    if (validRoom(sample) && sample.id === 'sample' && daysLeft(sample) > 0) app.sample = sample;
  } catch {
    /* A fresh demo remains available when storage is unavailable. */
  }
  app.ready = true;
}
export function saveRoom() {
  try {
    localStorage.setItem('z-split-room', JSON.stringify(app.room));
  } catch {
    notify('This browser cannot save your room. Keep this tab open.');
  }
}
let toastTimer: ReturnType<typeof setTimeout>;
export function notify(message: string) {
  app.toast = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = ''), 4200);
}
export function createRoom(
  values: Pick<Room, 'name' | 'count' | 'amount' | 'expiry' | 'giftCards' | 'split'>
) {
  app.room = {
    ...values,
    id: crypto.randomUUID().slice(0, 8),
    claimed: [],
    gifts: [],
    closed: false,
    funded: true,
    createdAt: Date.now(),
  };
  saveRoom();
}
export function claimLink(number: number) {
  return `${location.origin}/claim/${app.room.id}-${number}`;
}
export async function copyText(value: string, success = 'Copied. Ready to hand out!') {
  try {
    await navigator.clipboard.writeText(value);
    notify(success);
  } catch {
    notify('Clipboard unavailable. Select and copy the link shown on this page.');
  }
}
export function redeem(number: number, gift: boolean, sample = false) {
  const room = sample ? app.sample : app.room;
  if (room.closed || !room.funded || daysLeft(room) === 0 || room.claimed.includes(number))
    return false;
  room.claimed.push(number);
  if (gift) room.gifts.push(number);
  app.lastClaim = number;
  app.payout = gift ? 'gift' : 'wallet';
  if (sample) {
    try {
      sessionStorage.setItem('z-split-sample', JSON.stringify(room));
    } catch {
      notify('Your sample receipt is available in this tab while it stays open.');
    }
  } else saveRoom();
  return true;
}
