import { test } from 'node:test';
import assert from 'node:assert/strict';
import { issueBackendClaims, getBackendClaim, redeemBackendClaim } from '../src/lib/api.ts';

test('issued tickets use the actual backend claim_id in links', async (t) => {
  t.mock.method(globalThis, 'fetch', async () =>
    Response.json({
      tickets: [{ claim_id: 'first-claim', token: 'test-bearer', amount_zat: 80_000 }],
    })
  );
  const tickets = await issueBackendClaims('test-room');
  assert.equal(tickets[0].id, 'first-claim');
  assert.equal(tickets[0].token, 'test-bearer');
});

test('claim requests match the native API bearer authentication and redemption schema', async (t) => {
  t.mock.method(globalThis, 'fetch', async (url: string, options: RequestInit) => {
    assert.equal(new Headers(options.headers).get('Authorization'), 'Bearer test-ticket');
    if (url.endsWith('/redeem')) {
      assert.deepEqual(JSON.parse(String(options.body)), {
        ephemeral_receiver: 'utest1-test-receiver',
        idempotency_key: 'test-idempotency',
        client_network: 'testnet',
        wallet_persisted: true,
      });
      return Response.json({ claim_id: 'test-claim', status: 'funding_queued' }, { status: 202 });
    }
    return Response.json({ claim_id: 'test-claim', status: 'reserved' });
  });
  await getBackendClaim('test-claim', 'test-ticket');
  const result = await redeemBackendClaim(
    'test-claim',
    'test-ticket',
    'utest1-test-receiver',
    'test-idempotency',
    true
  );
  assert.equal(result.status, 'funding_queued');
});
