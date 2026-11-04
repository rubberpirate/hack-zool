#!/usr/bin/env node
// Public smoke checks. Optional room creation uses real API state, never synthetic funding.
import assert from 'node:assert/strict';

const origin = process.env.ZOOL_ORIGIN || 'https://zool.rubberpirate.in';
const token = process.env.ZOOL_ORGANIZER_TOKEN;
const headers = token ? { Authorization: `Bearer ${token}` } : {};
const checks = [];
async function expectStatus(path, expected, options = {}) {
  const response = await fetch(new URL(path, origin), {
    ...options,
    headers: { Origin: origin, ...options.headers },
    signal: AbortSignal.timeout(30_000),
  });
  assert.equal(response.status, expected, `${options.method || 'GET'} ${path}`);
  checks.push(`${options.method || 'GET'} ${path}: ${expected}`);
  return response;
}
await expectStatus('/', 200);
await expectStatus('/organizer', 200);
await expectStatus('/healthz', 200);
await expectStatus('/readyz', 200);
const metadata = await (await expectStatus('/api/v1/meta', 200)).json();
assert.equal(metadata.network, 'testnet');
assert.equal(metadata.sweep_reports_verified, false);
await expectStatus('/openapi.json', 200);
await expectStatus('/api/v1/rooms', 401, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: '{}',
});
await expectStatus('/organizer/session', 401, { method: 'POST' });
await expectStatus('/api/v1/frost/setup', 404, { method: 'POST', headers });
await expectStatus('/api/v1/rooms/00000000-0000-0000-0000-000000000000/dev-confirm', 404, {
  method: 'POST',
  headers,
});
await expectStatus('/docs', 404);
if (token) {
  await expectStatus('/organizer/session', 200, { method: 'POST', headers });
  if (process.argv.includes('--create-room')) {
    const response = await expectStatus('/api/v1/rooms', 201, {
      method: 'POST',
      headers: { ...headers, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        network: 'testnet',
        amounts_zat: [30_000],
        fee_reserve_zat: 10_000,
        expires_at: new Date(Date.now() + 86_400_000).toISOString(),
      }),
    });
    const room = await response.json();
    assert.equal(room.confirmed_zat, 0);
    assert.equal(room.claims_issued, false);
    const deposit = await (
      await expectStatus(`/api/v1/rooms/${room.id}/deposit`, 200, { headers })
    ).json();
    assert.ok(deposit.address.startsWith('utest1'));
    assert.equal(deposit.confirmed_zat, 0);
    console.log(JSON.stringify({ created_unfunded_room: room.id }));
  }
}
console.log(JSON.stringify({ origin, passed: checks.length, checks }, null, 2));
