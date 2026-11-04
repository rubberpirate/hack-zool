import { test, expect } from '@playwright/test';

test('organizer creates a tiered room, exports real PDF, recipient claims once, dashboard updates and room refunds', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await page.getByRole('link', { name: "I'm paying a room" }).click();
  await page.getByRole('button', { name: 'Fund and make links' }).click();
  await expect(page.getByRole('alert')).toContainText('Give your room a name');
  await page.getByLabel('ROOM NAME', { exact: true }).fill('Friday builders');
  await page.getByLabel('HOW MANY CLAIM LINKS').fill('13');
  await page.getByLabel('ZEC PER CLAIM').fill('0.50');
  await page.getByRole('button', { name: 'TIERS:' }).click();
  await expect(page.locator('.deposit-total')).toHaveText('8.50 ZEC');
  await page.getByRole('button', { name: 'Fund and make links' }).click();
  await expect(page).toHaveURL(/room\/fund/);
  await page.getByRole('button', { name: 'Fund demo pool' }).click();
  await expect(page).toHaveURL(/room\/claims/);
  await expect(page.locator('.qr-card')).toHaveCount(12);
  await page.evaluate(() => {
    window.print = () => {
      document.documentElement.dataset.printedQr = String(
        document.querySelectorAll('.qr-grid .qr-image svg').length
      );
    };
  });
  await page.getByRole('button', { name: 'PRINT SHEET' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-printed-qr', '13');
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'DOWNLOAD PDF' }).click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/\.pdf$/);
  const { readFile } = await import('node:fs/promises');
  const contents = await readFile((await file.path())!);
  expect(contents.subarray(0, 8).toString()).toBe('%PDF-1.4');
  await file.saveAs('test-results/claims.pdf');
  await page.getByRole('button', { name: 'Next page' }).click();
  await expect(page.locator('.qr-card')).toHaveCount(1);
  await page.getByRole('button', { name: 'Previous page' }).click();
  const link = page.locator('.qr-card').first();
  const href = await link.getAttribute('href');
  await link.click();
  await expect(page.locator('h1')).toContainText('1.50 ZEC.');
  await page.getByRole('link', { name: 'Claim it' }).click();
  await page.getByRole('button', { name: 'Sweep 1.50 ZEC' }).click();
  await expect(page.getByRole('alert')).toContainText('shielded address');
  await page.getByRole('button', { name: 'SCAN', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: 'Use demo address' }).click();
  await page.getByRole('button', { name: 'Sweep 1.50 ZEC' }).click();
  await expect(page).toHaveURL(/\/done$/);
  await expect(page.locator('h1')).toContainText('YOURS.');
  const storage = await page.evaluate(() => localStorage.getItem('z-split-room'));
  expect(storage).not.toContain('walletaddress');
  expect(JSON.parse(storage!).claimed).toEqual([1]);
  await page.goto(href!);
  await expect(page.locator('h1')).toHaveText('THIS LINK IS SPENT.');
  await page.goto('/room/live');
  await expect(page.locator('.live-heading')).toContainText('1 OF 13');
  await expect(page.locator('.remaining-card strong')).toHaveText('7.00 ZEC');
  await page.getByRole('button', { name: 'Close room and refund pool' }).click();
  await page.getByRole('button', { name: 'Close and refund', exact: true }).click();
  await expect(page.locator('.live-heading')).toContainText('CLOSED.');
  await page.goto('/room/claims');
  await expect(page.locator('.status-chip').filter({ hasText: 'CLOSED' })).toHaveCount(11);
  await page.locator('.qr-card').nth(1).click();
  await expect(page.locator('h1')).toHaveText('ROOM CLOSED.');
  expect(errors).toEqual([]);
});

test('gift card payout, reload, spent links and invalid links work', async ({ page }) => {
  await page.goto('/claim/demo');
  await page.getByRole('link', { name: 'Claim it' }).click();
  await page.getByRole('button', { name: 'A GIFT CARD' }).click();
  await page.getByLabel('PICK YOUR LITTLE TREAT').selectOption('Steam');
  await page.getByRole('button', { name: 'Get Steam gift card' }).click();
  await expect(page.locator('h1')).toContainText('YOURS.');
  await expect(page.locator('.gift-code')).toBeVisible();
  await page.reload();
  await expect(page.locator('.gift-code')).toBeVisible();
  await page.goto('/claim/unknown-link');
  await expect(page.locator('h1')).toHaveText('LINK NOT FOUND.');
});

test('all reference routes fit mobile, tablet and desktop with reduced motion', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const width of [320, 390, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    for (const path of [
      '/',
      '/room/new',
      '/room/claims',
      '/room/live',
      '/claim/demo',
      '/claim/demo/payout',
    ]) {
      await page.goto(path);
      await page.evaluate(() => document.fonts.ready);
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
        `${path} at ${width}`
      ).toBeLessThanOrEqual(width);
      await expect(page.locator('main h1')).toBeVisible();
    }
  }
  await page.getByRole('button', { name: 'MENU', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#main-menu')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('#main-menu')).toHaveCount(0);
});

test('unfunded rooms return to funding and expired claim links are inactive', async ({ page }) => {
  await page.goto('/room/new');
  await page.getByLabel('ROOM NAME', { exact: true }).fill('Not quite funded');
  await page.getByRole('button', { name: 'Fund and make links' }).click();
  await expect(page).toHaveURL(/room\/fund/);
  await page.goto('/room/claims');
  await expect(page).toHaveURL(/room\/fund/);
  await page.getByRole('button', { name: 'Fund demo pool' }).click();
  await expect(page).toHaveURL(/room\/claims/);
  const href = await page.locator('.qr-card').first().getAttribute('href');
  await page.evaluate(() => {
    const room = JSON.parse(localStorage.getItem('z-split-room')!);
    room.createdAt = Date.now() - 8 * 86400000;
    localStorage.setItem('z-split-room', JSON.stringify(room));
  });
  await page.reload();
  await expect(page.locator('.status-chip').first()).toHaveText('EXPIRED');
  await expect(page.locator('.qr-image')).toHaveCount(0);
  await page.goto(href!);
  await expect(page.locator('h1')).toHaveText('LINK EXPIRED.');
});
