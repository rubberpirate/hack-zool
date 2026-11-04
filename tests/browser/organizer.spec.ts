import { test, expect } from '@playwright/test';

test('organizer access rejects a bad token before saving a valid credential', async ({ page }) => {
  await page.route('**/organizer/session', (route) =>
    route.fulfill({
      status: route.request().headers().authorization === 'Bearer test-organizer-token' ? 200 : 401,
      json: {
        authenticated: route.request().headers().authorization === 'Bearer test-organizer-token',
      },
    })
  );
  await page.goto('/organizer');
  await page.getByLabel('ORGANIZER TOKEN', { exact: true }).fill('bad-token');
  await page.getByRole('button', { name: 'Open organizer tools' }).click();
  await expect(page.getByRole('alert')).toContainText('not accepted');
  expect(await page.evaluate(() => localStorage.getItem('zsplit-auth-token'))).toBeNull();
  await page.getByLabel('ORGANIZER TOKEN', { exact: true }).fill('test-organizer-token');
  await page.getByRole('button', { name: 'Open organizer tools' }).click();
  await expect(page).toHaveURL(/\/room\/new$/);
  expect(await page.evaluate(() => localStorage.getItem('zsplit-auth-token'))).toBe(
    'test-organizer-token'
  );
});
