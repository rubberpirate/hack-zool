import { test, expect, type Page } from '@playwright/test';

const roomId = '00000000-0000-4000-8000-000000000099';
const recipients = [
  { rank: '1st Place', email: 'winner@example.invalid', prize: 0.001, required: true },
  { rank: '2nd Place', email: '', prize: 0.0005, required: false },
];

function captureErrors(page: Page) {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  return errors;
}

async function stubDeposit(page: Page) {
  await page.route(`**/api/v1/rooms/${roomId}/deposit`, (route) =>
    route.fulfill({
      json: {
        room_id: roomId,
        network: 'testnet',
        address: 'utest1-browser-regression-fixture',
        confirmed_zat: 0,
        spendable_zat: 0,
        required_zat: 170_000,
        is_ready: false,
        birthday_height: 1,
        scanned_height: 1,
        chain_height: 1,
      },
    })
  );
}

test('funding survives restoring recipients from browser storage and reloading', async ({
  page,
}) => {
  const errors = captureErrors(page);
  await stubDeposit(page);
  await page.addInitScript(
    ({ roomId, recipients }) => {
      localStorage.setItem('zool-recipients', JSON.stringify(recipients));
      localStorage.setItem(
        'zool-room',
        JSON.stringify({
          id: roomId,
          name: 'Saved winners',
          count: 2,
          amount: 0.0001,
          expiry: 7,
          split: 'tiers',
          giftCards: false,
          funded: false,
          claimed: [],
          gifts: [],
          closed: false,
          createdAt: Date.now(),
        })
      );
      localStorage.setItem('zool-backend-room', JSON.stringify({ id: roomId }));
    },
    { roomId, recipients }
  );

  await page.goto('/room/fund');
  await expect(page.getByRole('heading', { name: 'FUND THE ROOM.', exact: true })).toBeVisible();
  await expect(page.locator('.pool-ticket')).toContainText('2nd Place');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'FUND THE ROOM.', exact: true })).toBeVisible();
  await expect(page.locator('.live-status-card')).toContainText('Awaiting Shielded Deposit');
  expect(errors).toEqual([]);
});

test('room creation navigates to funding with reactive recipient data', async ({ page }) => {
  const errors = captureErrors(page);
  await stubDeposit(page);
  await page.route('**/api/v1/rooms', async (route) => {
    const request = route.request().postDataJSON();
    await route.fulfill({
      status: 201,
      json: {
        id: roomId,
        network: 'testnet',
        status: 'awaiting_funds',
        amounts_zat: request.amounts_zat,
        fee_budget_per_ticket_zat: 10_000,
        confirmed_zat: 0,
        reserved_zat: 0,
        claims_issued: false,
        expires_at: request.expires_at,
      },
    });
  });

  await page.goto('/room/new');
  await page.getByLabel('ROOM NAME', { exact: true }).fill('New winners');
  await page.getByRole('button', { name: 'Generate Coordinator Address' }).click();
  await expect(page).toHaveURL(/\/room\/fund$/);
  await expect(page.getByRole('heading', { name: 'FUND THE ROOM.', exact: true })).toBeVisible();
  await expect(page.locator('.pool-ticket')).toContainText('3rd Place');
  await expect(page.locator('.addr-box code')).toHaveText('utest1-browser-regression-fixture');
  expect(errors).toEqual([]);
});
