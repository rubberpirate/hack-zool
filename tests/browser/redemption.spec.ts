import { test, expect } from '@playwright/test';

test('claim redemption works without randomUUID, authenticates, and reuses its key after a timeout', async ({
  page,
}) => {
  const claimId = '11111111-1111-4111-8111-111111111111';
  const bearer = 'test-only-bearer';
  const receiver = `utest1${'q'.repeat(90)}`;
  let queued = false;
  const attempts: string[] = [];
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript(() => {
    Object.defineProperty(globalThis.crypto, 'randomUUID', {
      configurable: true,
      value: undefined,
    });
  });
  await page.route(new RegExp(`/api/v1/claims/${claimId}(?:/status)?$`), (route) => {
    expect(route.request().headers().authorization).toBe(`Bearer ${bearer}`);
    return route.fulfill({
      json: {
        claim_id: claimId,
        network: 'testnet',
        amount_zat: 100_000,
        status: queued ? 'funding_queued' : 'reserved',
        expires_at: new Date(Date.now() + 3600000).toISOString(),
      },
    });
  });
  await page.route(`**/api/v1/claims/${claimId}/redeem`, async (route) => {
    expect(route.request().headers().authorization).toBe(`Bearer ${bearer}`);
    const body = route.request().postDataJSON();
    expect(body).toEqual({
      ephemeral_receiver: receiver,
      idempotency_key: expect.any(String),
      client_network: 'testnet',
      wallet_persisted: true,
    });
    expect(body.idempotency_key).toMatch(
      /^[\da-f]{8}-[\da-f]{4}-4[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/
    );
    attempts.push(body.idempotency_key);
    if (attempts.length === 1) {
      return route.fulfill({ status: 503, json: { message: 'Wallet temporarily unavailable' } });
    }
    queued = true;
    return route.fulfill({ status: 202, json: { claim_id: claimId, status: 'funding_queued' } });
  });
  await page.goto(`/claim/${claimId}#t=${bearer}`);
  await expect(page.getByRole('link', { name: 'Claim it', exact: true })).toHaveAttribute(
    'href',
    `/claim/${claimId}/payout#t=${bearer}`
  );
  await page.getByRole('link', { name: 'Claim it', exact: true }).click();
  await page.locator('#wallet-address').fill(receiver);
  await page.getByRole('checkbox', { name: /separate claim wallet/ }).check();
  await page.getByRole('button', { name: /^Sweep/ }).click();
  await expect(page.getByRole('alert')).toContainText('Wallet temporarily unavailable');
  await page.getByRole('button', { name: /^Sweep/ }).click();
  await expect(page).toHaveURL(new RegExp(`/claim/${claimId}/done#t=${bearer}$`));
  await expect(page.getByRole('heading', { name: /PAYOUT.*QUEUED/ })).toBeVisible();
  expect(attempts).toHaveLength(2);
  expect(attempts[0]).toBe(attempts[1]);
  expect(errors).toEqual([]);
});
