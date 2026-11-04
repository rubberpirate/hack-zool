import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser',
  use: { baseURL: 'http://localhost:5174', headless: true, channel: 'chromium' },
  reporter: 'list',
});
