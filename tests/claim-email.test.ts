import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dispatchClaimEmails, parseRecipients } from '../src/lib/server/claim-email.ts';

const recipients = parseRecipients([
  {
    email: 'first@example.invalid',
    number: 1,
    amountZec: '0.0008',
    prizeLabel: '1st Place',
    claimUrl: 'https://example.invalid/claim/first#t=test-first',
  },
  {
    email: 'second@example.invalid',
    number: 2,
    amountZec: '0.0005',
    prizeLabel: '2nd Place',
    claimUrl: 'https://example.invalid/claim/second#t=test-second',
  },
]);
const config = {
  apiKey: 'test-key',
  fromEmail: 'Zool <onboarding@resend.dev>',
  roomName: '<Room & winners>',
};

test('provider rejection is a failure with its actionable reason, never zero-delivered success', async () => {
  const result = await dispatchClaimEmails(recipients, config, async () =>
    Response.json(
      { message: 'You can only send testing emails to your own email address.' },
      { status: 403 }
    )
  );
  assert.equal(result.success, false);
  assert.equal(result.accepted, 0);
  assert.equal(result.failed, 2);
  assert.match(result.error!, /verified Resend domain/);
  assert.ok(result.results.every((r) => !r.success && r.error?.includes('testing emails')));
});

test('each email carries its own CID PNG and claim, with stable per-email idempotency', async () => {
  const keys: string[] = [];
  let calls = 0;
  const fetcher: typeof fetch = async (url, options) => {
    assert.equal(url, 'https://api.resend.com/emails');
    keys.push(new Headers(options?.headers).get('Idempotency-Key')!);
    const message = JSON.parse(String(options?.body));
    const index = calls++ % recipients.length;
    assert.deepEqual(message.to, [recipients[index].email]);
    assert.match(message.html, /&lt;Room &amp; winners&gt;/);
    assert.ok(message.html.includes(recipients[index].claimUrl));
    assert.ok(!message.html.includes(recipients[1 - index].claimUrl));
    assert.match(message.html, /src="cid:claim-qr"/);
    assert.ok(!message.html.includes('src="data:'));
    const attachment = message.attachments[0];
    assert.equal(attachment.content_id, 'claim-qr');
    assert.equal(attachment.content_type, 'image/png');
    const png = Buffer.from(attachment.content, 'base64');
    assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    assert.ok(png.readUInt32BE(16) >= 240);
    return Response.json({ id: index === 0 ? 'provider-first' : 'provider-second' });
  };
  const result = await dispatchClaimEmails(recipients, config, fetcher);
  await dispatchClaimEmails(recipients, config, fetcher);
  assert.equal(result.success, true);
  assert.equal(result.accepted, 2);
  assert.equal(result.failed, 0);
  assert.equal(result.results[0].id, 'provider-first');
  assert.equal(keys[0], keys[2]);
  assert.equal(keys[1], keys[3]);
  assert.notEqual(keys[0], keys[1]);
});

test('missing message IDs and network uncertainty never report full acceptance', async () => {
  let calls = 0;
  const missing = await dispatchClaimEmails(recipients, config, async () =>
    Response.json(calls++ === 0 ? { id: 'only-first' } : {})
  );
  assert.equal(missing.success, false);
  assert.equal(missing.accepted, 1);
  assert.equal(missing.failed, 1);
  const network = await dispatchClaimEmails(recipients, config, async () => {
    throw new Error('timeout');
  });
  assert.equal(network.success, false);
  assert.match(network.error!, /Check the Resend logs before retrying/);
});

test('invalid recipients and unsafe claim URLs are rejected before calling the provider', () => {
  assert.throws(() => parseRecipients([]));
  for (const patch of [
    { email: 'not-email' },
    { claimUrl: 'javascript:alert(1)' },
    { amountZec: '-1' },
    { number: 0 },
  ]) {
    assert.throws(() => parseRecipients([{ ...recipients[0], ...patch }]));
  }
});
