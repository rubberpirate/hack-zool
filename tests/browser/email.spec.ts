import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      'zool-room',
      JSON.stringify({
        id: 'email-regression',
        name: 'Email regression',
        count: 3,
        amount: 0.01,
        expiry: 7,
        split: 'tiers',
        giftCards: false,
        claimed: [],
        gifts: [],
        funded: true,
        closed: false,
        createdAt: Date.now(),
      })
    );
    localStorage.setItem(
      'zool-recipients',
      JSON.stringify([
        { rank: '1st Place', email: 'first@example.invalid', prize: 0.0008 },
        { rank: '2nd Place', email: 'second@example.invalid', prize: 0.0005 },
        { rank: '3rd Place', email: 'third@example.invalid', prize: 0.0003 },
      ])
    );
    localStorage.setItem(
      'zool-issued-tickets',
      JSON.stringify([
        { claim_id: 'first-claim', token: 'first-bearer', amount_zat: 80_000 },
        { claim_id: 'second-claim', token: 'second-bearer', amount_zat: 50_000 },
        { claim_id: 'third-claim', token: 'third-bearer', amount_zat: 30_000 },
      ])
    );
  });
});

for (const scenario of [
  {
    name: 'provider rejection',
    status: 502,
    body: { success: false, accepted: 0, failed: 3, error: 'Verify your sender domain in Resend.' },
    message: /0 of 3 emails accepted.*Verify your sender domain/,
  },
  {
    name: 'legacy zero-delivered response',
    status: 200,
    body: { success: true, delivered: 0, failed: 3 },
    message: /0 of 3 emails accepted/,
  },
  {
    name: 'partial acceptance',
    status: 200,
    body: { success: false, accepted: 1, failed: 2, error: 'Two emails were not accepted.' },
    message: /1 of 3 emails accepted.*Two emails were not accepted/,
  },
]) {
  test(`${scenario.name} is shown as a failure, never green success`, async ({ page }) => {
    await page.route('**/resend-email', (route) =>
      route.fulfill({ status: scenario.status, json: scenario.body })
    );
    await page.goto('/room/claims');
    await expect(page.getByRole('heading', { name: '3 CLAIM LINKS, READY.' })).toBeVisible();
    await page.getByRole('button', { name: /SEND TO RECIPIENTS/ }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog.locator('input[type=email]')).toHaveCount(3);
    await dialog.getByRole('button', { name: /Send Claim Emails via Resend/ }).click();
    await expect(dialog.getByRole('alert')).toContainText(scenario.message);
    await expect(dialog).not.toContainText('Successfully delivered');
    await expect(dialog.getByRole('status')).toHaveCount(0);
  });
}

test('all accepted emails are reported as pending inbox delivery without exposing the API key', async ({
  page,
}) => {
  await page.route('**/resend-email', async (route) => {
    const body = route.request().postDataJSON();
    expect(route.request().headers().authorization).toMatch(/^Bearer .+/);
    expect(body).not.toHaveProperty('apiKey');
    expect(body.recipients).toHaveLength(3);
    expect(body.recipients[0].claimUrl).toBe(
      'http://localhost:5174/claim/first-claim#t=first-bearer'
    );
    expect(body.recipients.map((r: { amountZec: string }) => r.amountZec)).toEqual([
      '0.0008',
      '0.0005',
      '0.0003',
    ]);
    await route.fulfill({ json: { success: true, accepted: 3, failed: 0, results: [] } });
  });
  await page.goto('/room/claims');
  await expect(page.getByRole('heading', { name: '3 CLAIM LINKS, READY.' })).toBeVisible();
  await page.getByRole('button', { name: /SEND TO RECIPIENTS/ }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: /Send Claim Emails via Resend/ }).click();
  await expect(dialog.getByRole('status')).toContainText(
    'Resend accepted 3 of 3 emails. Inbox delivery is pending.'
  );
});
