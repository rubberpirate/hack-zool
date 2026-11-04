import { test, expect } from '@playwright/test';

test('every hero illustration supports picking up, returning and keyboard play', async ({
  page,
}) => {
  await page.goto('/');
  const toys = page.locator('.scene-toy');
  await expect(toys).toHaveCount(12);
  for (let i = 0; i < 12; i++) {
    const toy = toys.nth(i);
    await expect(toy).toBeEnabled();
    await toy.focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('.marketing-page [role=status]')).toContainText('bounced back');
  }
  const coin = page.getByRole('button', { name: 'Play with zec coin', exact: true });
  await coin.scrollIntoViewIfNeeded();
  const box = await coin.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  await page.mouse.down();
  await page.mouse.move(box!.x + box!.width / 2 + 80, box!.y + box!.height / 2 - 90, { steps: 10 });
  await expect(coin).toHaveAttribute('data-dragging', 'true');
  expect(
    await coin.locator('.scene-piece').evaluate((el) => getComputedStyle(el).transform)
  ).not.toBe('none');
  await page.mouse.up();
  await expect(coin).not.toHaveAttribute('data-dragging');
  await expect
    .poll(async () =>
      coin.locator('.scene-piece').evaluate((el) => {
        const m = new DOMMatrix(getComputedStyle(el).transform);
        return Math.abs(m.m41) + Math.abs(m.m42);
      })
    )
    .toBeLessThan(1);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await coin.focus();
  await page.keyboard.press('Enter');
  expect(await coin.locator('.scene-piece').evaluate((el) => getComputedStyle(el).transform)).toBe(
    'none'
  );
});

test('claim entry handles invalid links and opens the complete sample flow', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('link', { name: 'I have a claim link', exact: true }).click();
  await expect(page).toHaveURL(/\/claim$/);
  await page.getByLabel('YOUR CLAIM LINK').fill('not-a-claim');
  await page.getByRole('button', { name: 'Open my claim' }).click();
  await expect(page.getByRole('alert')).toContainText('complete Z-Split claim link');
  await page.getByLabel('YOUR CLAIM LINK').fill('https://example.test/claim/demo');
  await page.getByRole('button', { name: 'Open my claim' }).click();
  await expect(page).toHaveURL(/\/claim\/demo$/);
  await expect(page.locator('h1')).toContainText("YOU'VE GOT");
});

test('product story, FAQ, gift-card shortcut and added pages are connected', async ({ page }) => {
  await page.goto('/');
  await page.locator('.scroll-cue').click();
  await expect(page).toHaveURL(/#how-it-works$/);
  const question = page
    .locator('summary')
    .filter({ hasText: 'What if someone doesn’t have a wallet?' });
  await question.click();
  await expect(page.locator('details[open]')).toContainText('Enable gift cards');
  await page.getByRole('link', { name: /TRY THE GIFT CARD FLOW/ }).click();
  await expect(page.locator('.payout-option.active')).toContainText('A GIFT CARD');
  for (const [path, title] of [
    ['/rooms', 'YOUR ROOM.'],
    ['/how-it-works', 'A LITTLE POOL.'],
    ['/privacy', 'LESS TO COLLECT.'],
  ]) {
    await page.goto(path);
    await expect(page.locator('h1')).toContainText(title);
    await expect(page.locator('.site-footer')).toBeVisible();
  }
});

test('compact desktop scaling and new screens fit mobile through ultrawide', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  for (const width of [320, 390, 768, 1440, 1920, 2560]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const path of ['/', '/rooms', '/claim', '/how-it-works', '/privacy']) {
      await page.goto(path);
      await page.evaluate(() => document.fonts.ready);
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
        `${path} at ${width}`
      ).toBeLessThanOrEqual(width);
      await expect(page.locator('main h1')).toBeVisible();
    }
    if (width >= 1440)
      expect(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize)).toBe(
        '14px'
      );
  }
  expect(errors).toEqual([]);
});

test('sample gift claims remain available without changing a closed organizer room', async ({
  page,
}) => {
  await page.goto('/room/new');
  await expect(page.getByRole('button', { name: 'Fund and make links' })).toBeEnabled();
  await page.getByLabel('ROOM NAME', { exact: true }).fill('Keep this room');
  await page.getByRole('button', { name: 'Fund and make links' }).click();
  await page.getByRole('button', { name: 'Fund demo pool' }).click();
  await expect(page).toHaveURL(/room\/claims/);
  await page.evaluate(() => {
    const room = JSON.parse(localStorage.getItem('z-split-room')!);
    room.closed = true;
    room.giftCards = false;
    localStorage.setItem('z-split-room', JSON.stringify(room));
  });
  await page.goto('/');
  await page.reload();
  await page.getByRole('link', { name: /TRY THE GIFT CARD FLOW/ }).click();
  await expect(page.locator('.payout-option.active')).toContainText('A GIFT CARD');
  await page.getByRole('button', { name: 'Get Amazon gift card' }).click();
  await expect(page).toHaveURL(/\/claim\/sample-\d+\/done$/);
  await expect(page.locator('.gift-code')).toBeVisible();
  const current = await page.evaluate(() => JSON.parse(localStorage.getItem('z-split-room')!));
  expect(current.name).toBe('Keep this room');
  expect(current.closed).toBe(true);
  expect(current.giftCards).toBe(false);
  expect(current.claimed).toEqual([]);
  await page.reload();
  await expect(page.locator('.gift-code')).toBeVisible();
});
