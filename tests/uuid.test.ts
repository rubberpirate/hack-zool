import { test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUuid } from '../src/lib/uuid.ts';

test('UUID generation uses secure browser randomness without randomUUID', (t) => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'crypto')!;
  const nativeCrypto = globalThis.crypto;
  Object.defineProperty(globalThis, 'crypto', {
    configurable: true,
    value: {
      getRandomValues: nativeCrypto.getRandomValues.bind(nativeCrypto),
    },
  });
  t.after(() => Object.defineProperty(globalThis, 'crypto', original));
  const values = Array.from({ length: 50 }, () => randomUuid());
  assert.equal(new Set(values).size, values.length);
  for (const value of values)
    assert.match(value, /^[\da-f]{8}-[\da-f]{4}-4[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/);
});

test('UUID generation fails clearly when no secure entropy source exists', (t) => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'crypto')!;
  Object.defineProperty(globalThis, 'crypto', { configurable: true, value: undefined });
  t.after(() => Object.defineProperty(globalThis, 'crypto', original));
  assert.throws(randomUuid, /Secure randomness is unavailable/);
});
