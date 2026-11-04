# Z-Split

A responsive Svelte 5 / SvelteKit 3 frontend recreated from the seven images in `wireframe/`. TypeScript, Tailwind CSS v4, Motion, self-hosted fonts, and custom SVG artwork.

Requires Node.js 22.17 or newer.

```sh
npm install
npm run dev
```

Open the local URL printed by Vite (default `http://localhost:5173`).

## Screens

| Screen                                           | Route                                                   |
| ------------------------------------------------ | ------------------------------------------------------- |
| Product landing page + interactive illustrations | `/`                                                     |
| Current room overview                            | `/rooms`                                                |
| Paste a claim link                               | `/claim`                                                |
| Walkthrough + FAQs                               | `/how-it-works`                                         |
| Privacy explanation                              | `/privacy`                                              |
| Set up the room                                  | `/room/new`                                             |
| Demo funding                                     | `/room/fund`                                            |
| Claim links                                      | `/room/claims`                                          |
| Live room                                        | `/room/live`                                            |
| Scan / claim introduction                        | `/claim/demo`                                           |
| Choose payout                                    | `/claim/demo/payout`                                    |
| Done                                             | `/claim/{room-id}-{claim-number}/done` after redemption |

The demo claim sheet starts with 3 of 50 links claimed. Opening the demo live room advances the mock event to the 37-claim snapshot shown in the reference. Create a room to start a fresh pool. Select a QR card or scan its real QR code to open its individual claim URL. Recipients can paste a shielded-address-shaped value or use the explicitly labeled demo scanner, choose a wallet or gift card payout, and complete a mock redemption. Used claims cannot be redeemed again. The dashboard and claim sheet share persisted room state and update across tabs. PDF export produces an actual vector PDF containing every claim; printing uses the browser's print dialog.

## Validation

```sh
npm run check
npm test
npm run build
npx playwright install chromium
npx playwright test
```

Browser tests expect a dev server on port 5174. Change `baseURL` in `playwright.config.ts` or start it using `npm run dev -- --port 5174`.

Room metadata and claimed flags persist in localStorage. Recipient wallet addresses are never persisted. Delete the `z-split-room` localStorage entry to restore the reference demo.

All transactions, scanning, gift cards, funding, and refunds are mock interactions. No wallet, Zcash, or gift-card service is connected. The browser's local state is demonstration state, not a secure redemption ledger. A production integration requires backend claim verification, real funding/redemption providers, and a deployment-specific SvelteKit adapter.

The framework setup follows the [official SvelteKit 3 migration guide](https://svelte.dev/docs/kit/migrating-to-sveltekit-3), with configuration in `vite.config.ts` and native `#lib` imports.

The bottom hero illustrations support mouse/touch dragging, hover tilt, and keyboard-triggered bounces. Each returns to its resting position with an interruptible Motion spring. Reduced-motion users get direct feedback without positional animation. Product sections use one-time scroll reveals; desktop sizing is capped instead of scaling with the viewport.

Sample claims (`/claim/demo` and `/claim/sample-{number}`) use independent session state. Trying either sample payout never changes the organizer’s current room, even when that room is closed or gift cards are disabled.
