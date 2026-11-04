export type Room = {
  id: string;
  name: string;
  count: number;
  amount: number;
  expiry: number;
  giftCards: boolean;
  split: 'equal' | 'tiers';
  claimed: number[];
  gifts: number[];
  closed: boolean;
  funded: boolean;
  createdAt: number;
};
export const liveDemoClaims = Array.from({ length: 50 }, (_, i) => i + 1).filter(
  (n) => ![4, 9, 13, 18, 22, 27, 31, 36, 40, 41, 44, 47, 50].includes(n)
);
export const demoRoom: Room = {
  id: 'demo',
  name: 'Builder bounties',
  count: 50,
  amount: 0.5,
  expiry: 7,
  giftCards: true,
  split: 'equal',
  claimed: [2, 7, 11],
  gifts: [],
  closed: false,
  funded: true,
  createdAt: Date.now() - 2 * 86_400_000,
};
export function claimAmount(room: Room, number: number) {
  return room.split === 'equal'
    ? room.amount
    : room.amount * (number === 1 ? 3 : number <= 3 ? 2 : 1);
}
export function poolTotal(room: Room) {
  return Array.from({ length: room.count }, (_, i) => claimAmount(room, i + 1)).reduce(
    (sum, n) => sum + n,
    0
  );
}
export function remainingTotal(room: Room) {
  return Array.from({ length: room.count }, (_, i) => i + 1)
    .filter((n) => !room.claimed.includes(n))
    .reduce((sum, n) => sum + claimAmount(room, n), 0);
}
export function daysLeft(room: Room, now = Date.now()) {
  return Math.max(0, Math.ceil((room.createdAt + room.expiry * 86_400_000 - now) / 86_400_000));
}
export function validRoom(room: unknown): room is Room {
  if (!room || typeof room !== 'object') return false;
  const r = room as Room;
  return (
    typeof r.id === 'string' &&
    typeof r.name === 'string' &&
    r.name.length <= 80 &&
    Number.isInteger(r.count) &&
    r.count >= 1 &&
    r.count <= 100 &&
    Number.isFinite(r.amount) &&
    r.amount >= 0.01 &&
    r.amount <= 1000 &&
    [1, 3, 7, 14, 30].includes(r.expiry) &&
    ['equal', 'tiers'].includes(r.split) &&
    typeof r.giftCards === 'boolean' &&
    typeof r.closed === 'boolean' &&
    typeof r.funded === 'boolean' &&
    Number.isFinite(r.createdAt) &&
    r.createdAt > 0 &&
    Array.isArray(r.claimed) &&
    Array.isArray(r.gifts) &&
    [...r.claimed, ...r.gifts].every((n) => Number.isInteger(n) && n >= 1 && n <= r.count) &&
    new Set(r.claimed).size === r.claimed.length &&
    new Set(r.gifts).size === r.gifts.length &&
    r.gifts.every((n) => r.claimed.includes(n))
  );
}
