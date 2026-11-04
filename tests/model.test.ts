import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  demoRoom,
  liveDemoClaims,
  poolTotal,
  remainingTotal,
  claimAmount,
  validRoom,
  daysLeft,
} from '../src/lib/model.ts';
test('the demo sheet and live snapshot have consistent totals', () => {
  const live = { ...demoRoom, claimed: liveDemoClaims, gifts: [2, 7, 11, 16, 23, 30] };
  assert.equal(demoRoom.claimed.length, 3);
  assert.equal(live.claimed.length, 37);
  assert.equal(new Set(live.claimed).size, 37);
  assert.equal(live.gifts.length, 6);
  assert.equal(poolTotal(demoRoom), 25);
  assert.equal(remainingTotal(live), 6.5);
  assert.ok(validRoom(demoRoom));
  assert.ok(validRoom(live));
});
test('tier totals and individual claims use the same amounts', () => {
  const room = { ...demoRoom, count: 5, split: 'tiers' as const, claimed: [1, 2] };
  assert.deepEqual(
    [1, 2, 3, 4, 5].map((n) => claimAmount(room, n)),
    [1.5, 1, 1, 0.5, 0.5]
  );
  assert.equal(poolTotal(room), 4.5);
  assert.equal(remainingTotal(room), 2);
});
test('storage validation rejects malformed amounts, counts, duplicate and out-of-range claims', () => {
  for (const patch of [
    { count: 0 },
    { count: 101 },
    { amount: -1 },
    { amount: NaN },
    { claimed: [1, 1] },
    { claimed: [51] },
    { gifts: [4] },
    { split: 'invalid' },
    { expiry: 2 },
    { giftCards: 'true' },
  ])
    assert.equal(validRoom({ ...demoRoom, ...patch }), false);
  assert.equal(validRoom(null), false);
  assert.equal(validRoom({}), false);
});

test('expiry counts down from funding and clamps expired links to zero', () => {
  const room = { ...demoRoom, createdAt: 1_000_000, expiry: 7 };
  assert.equal(daysLeft(room, 1_000_000), 7);
  assert.equal(daysLeft(room, 1_000_000 + 2 * 86_400_000), 5);
  assert.equal(daysLeft(room, 1_000_000 + 8 * 86_400_000), 0);
});

test('storage preserves unfunded rooms at the setup form minimum amount', () => {
  const room = { ...demoRoom, amount: 0.0001, funded: false };
  assert.equal(validRoom(room), true);
  assert.equal(validRoom({ ...room, amount: 0.00009999 }), false);
});
