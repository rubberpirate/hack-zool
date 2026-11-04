import { test, expect } from '@playwright/test';

const id = '22222222-2222-4222-8222-222222222222';
const bearer = 'test-only-status-token';
const txid = 'ab'.repeat(32);

test('payout polls automatically and shows broadcast and confirmation only when reported', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const states = [
    'funding_queued',
    'preparing',
    'funding_broadcast',
    'funded_unconfirmed',
    'funded_confirmed',
  ];
  let reads = 0;
  await page.route(`**/api/v1/claims/${id}/status`, (route) => {
    expect(route.request().headers().authorization).toBe(`Bearer ${bearer}`);
    const status = states[Math.min(reads++, states.length - 1)];
    return route.fulfill({
      json: {
        claim_id: id,
        amount_zat: 30_000,
        network: 'testnet',
        status,
        txid: ['funding_broadcast', 'funded_unconfirmed', 'funded_confirmed'].includes(status)
          ? txid
          : null,
        confirmations: status === 'funded_confirmed' ? 3 : status === 'funded_unconfirmed' ? 1 : 0,
      },
    });
  });
  await page.goto(`/claim/${id}/done#t=${bearer}`);
  await expect(page.getByRole('heading', { name: 'PAYOUT QUEUED.' })).toBeVisible();
  await expect(page.locator('.payment-spinner')).toBeVisible();
  await expect(page.getByLabel('Payout progress')).toContainText('Broadcast not confirmed yet');
  await expect(page.getByRole('button', { name: 'Mark as swept locally' })).toBeDisabled();
  await expect(page.getByRole('heading', { name: 'PREPARING PAYOUT.' })).toBeVisible({
    timeout: 7000,
  });
  await page.getByRole('button', { name: 'Refresh status', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'PAYMENT SENT.' })).toBeVisible();
  await expect(page.getByLabel('Payout progress')).toContainText('Transaction broadcast');
  await expect(page.locator('.payment-transaction code')).toHaveText(txid);
  await page.getByRole('button', { name: 'Refresh status', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'PAYMENT MINED.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Mark as swept locally' })).toBeDisabled();
  await page.getByRole('button', { name: 'Refresh status', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'PAYMENT CONFIRMED.' })).toBeVisible();
  await expect(page.locator('.payment-spinner')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Mark as swept locally' })).toBeEnabled();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  await page.screenshot({ path: '/tmp/zool-claim-confirmed-mobile.png', fullPage: true });
});

test('manual review is paused instead of pretending to send, and failed refreshes remain visible', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  let reads = 0;
  await page.route(`**/api/v1/claims/${id}/status`, (route) => {
    reads++;
    if (reads === 2)
      return route.fulfill({ status: 503, json: { message: 'Status service unavailable' } });
    return route.fulfill({
      json: {
        claim_id: id,
        amount_zat: 30_000,
        network: 'testnet',
        status: reads === 1 ? 'manual_review' : 'funding_queued',
        txid: null,
      },
    });
  });
  await page.goto(`/claim/${id}/done#t=${bearer}`);
  await expect(page.getByRole('heading', { name: 'PAYOUT PAUSED.' })).toBeVisible();
  await expect(page.getByRole('status', { name: 'Payout status' })).toContainText('Contact your organizer');
  await expect(page.locator('.payment-spinner')).toHaveCount(0);
  await expect(page.getByLabel('Payout progress')).toContainText('Broadcast not confirmed yet');
  await expect(page.getByRole('button', { name: 'Mark as swept locally' })).toBeDisabled();
  await page.screenshot({ path: '/tmp/zool-claim-paused-mobile.png', fullPage: true });
  await page.getByRole('button', { name: 'Refresh status', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Status service unavailable');
  await expect(page.getByRole('heading', { name: 'PAYOUT PAUSED.' })).toBeVisible();
  await page.getByRole('button', { name: 'Refresh status', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'PAYOUT QUEUED.' })).toBeVisible();
  await expect(page.getByRole('alert')).toHaveCount(0);
  await expect(page.locator('.payment-spinner')).toBeVisible();
});
