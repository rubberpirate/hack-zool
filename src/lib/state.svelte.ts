import { demoRoom, validRoom, daysLeft, type Room } from './model';
import {
  INITIAL_REPO_POOLS,
  DEMO_VERIFIED_INVOICES,
  ZEC_PRICE_USD,
  type RepoPool,
  type BitrefillInvoice,
} from './bitrefill';

export type ContributorNotification = {
  id: string;
  contributorGithub: string;
  repo: string;
  prNumber: number;
  brand: string;
  amountUsd: number;
  voucherCode: string;
  claimUrl: string;
  timestamp: number;
  read: boolean;
  bitrefillInvoiceId?: string;
};

export const app = $state({
  room: structuredClone(demoRoom),
  sample: {
    ...structuredClone(demoRoom),
    id: 'sample',
    claimed: [] as number[],
    gifts: [] as number[],
    createdAt: Date.now(),
  },
  ready: false,
  toast: '',
  payout: 'wallet' as 'wallet' | 'gift',
  giftBrand: 'Amazon',
  lastClaim: 4,
  activeGithubUser: 'alex-dev',
  repoPools: structuredClone(INITIAL_REPO_POOLS) as Record<string, RepoPool>,
  invoices: structuredClone(DEMO_VERIFIED_INVOICES) as BitrefillInvoice[],
  contributorNotifications: [
    {
      id: 'notif-demo-1',
      contributorGithub: 'alex-dev',
      repo: 'privacy-scaling-explorations/zk-pool',
      prNumber: 42,
      brand: 'Amazon',
      amountUsd: 25,
      voucherCode: 'BITREFILL-AMZN-9824-ZEC',
      claimUrl: '/claim/sample-4',
      timestamp: Date.now() - 3600000,
      read: false,
      bitrefillInvoiceId: 'inv_bitrefill_live_89234a71',
    }
  ] as ContributorNotification[],
});
export function loadRoom() {
  try {
    const stored = JSON.parse(localStorage.getItem('z-split-room') || 'null');
    if (validRoom(stored)) app.room = stored;
    const sample = JSON.parse(sessionStorage.getItem('z-split-sample') || 'null');
    if (validRoom(sample) && sample.id === 'sample' && daysLeft(sample) > 0) app.sample = sample;
  } catch {
    /* A fresh demo remains available when storage is unavailable. */
  }
  app.ready = true;
}
export function saveRoom() {
  try {
    localStorage.setItem('z-split-room', JSON.stringify(app.room));
  } catch {
    notify('This browser cannot save your room. Keep this tab open.');
  }
}
let toastTimer: ReturnType<typeof setTimeout>;
export function notify(message: string) {
  app.toast = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = ''), 4200);
}
export function createRoom(
  values: Pick<Room, 'name' | 'count' | 'amount' | 'expiry' | 'giftCards' | 'split'>
) {
  app.room = {
    ...values,
    id: crypto.randomUUID().slice(0, 8),
    claimed: [],
    gifts: [],
    closed: false,
    funded: true,
    createdAt: Date.now(),
  };
  saveRoom();
}
export function claimLink(number: number) {
  return `${location.origin}/claim/${app.room.id}-${number}`;
}
export async function copyText(value: string, success = 'Copied. Ready to hand out!') {
  try {
    await navigator.clipboard.writeText(value);
    notify(success);
  } catch {
    notify('Clipboard unavailable. Select and copy the link shown on this page.');
  }
}
export function redeem(number: number, gift: boolean, sample = false) {
  const room = sample ? app.sample : app.room;
  if (room.closed || !room.funded || daysLeft(room) === 0 || room.claimed.includes(number))
    return false;
  room.claimed.push(number);
  if (gift) room.gifts.push(number);
  app.lastClaim = number;
  app.payout = gift ? 'gift' : 'wallet';
  if (sample) {
    try {
      sessionStorage.setItem('z-split-sample', JSON.stringify(room));
    } catch {
      notify('Your sample receipt is available in this tab while it stays open.');
    }
  } else saveRoom();
  return true;
}

export function addContributorNotification(notif: Omit<ContributorNotification, 'id' | 'timestamp' | 'read'>) {
  const newNotif: ContributorNotification = {
    ...notif,
    id: `notif-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    timestamp: Date.now(),
    read: false,
  };
  app.contributorNotifications = [newNotif, ...app.contributorNotifications];
  notify(`Voucher sent to @${notif.contributorGithub}! Notification delivered in notification bar.`);
}

export function markNotificationRead(id: string) {
  const target = app.contributorNotifications.find((n) => n.id === id);
  if (target) target.read = true;
}

export function dismissNotification(id: string) {
  app.contributorNotifications = app.contributorNotifications.filter((n) => n.id !== id);
}

export function getOrCreateRepoPool(repo: string): RepoPool {
  if (!app.repoPools[repo]) {
    app.repoPools[repo] = {
      repo,
      name: `${repo.split('/')[1] || repo} Contributor Pool`,
      poolBalanceZec: 15.0,
      poolBalanceUsd: 15.0 * ZEC_PRICE_USD,
      maxLimitPerPrUsd: 50.0,
      totalSpentUsd: 0,
      totalVouchersIssued: 0,
      maintainers: ['maintainer'],
      shieldedAddress: `u1zsplitpool${Math.random().toString(36).slice(2, 12)}psef8923489jksd0234293849234823904823094823904`,
    };
  }
  return app.repoPools[repo];
}

export function deductFromRepoPool(repo: string, amountUsd: number, invoice: BitrefillInvoice): boolean {
  const pool = getOrCreateRepoPool(repo);
  const amountZec = amountUsd / ZEC_PRICE_USD;

  if (pool.poolBalanceUsd < amountUsd) {
    notify(`Repo pool funds insufficient: has $${pool.poolBalanceUsd.toFixed(2)}, needs $${amountUsd}`);
    return false;
  }

  pool.poolBalanceUsd = Math.max(0, pool.poolBalanceUsd - amountUsd);
  pool.poolBalanceZec = Math.max(0, pool.poolBalanceZec - amountZec);
  pool.totalSpentUsd += amountUsd;
  pool.totalVouchersIssued += 1;

  app.invoices = [invoice, ...app.invoices];
  return true;
}

export function depositToRepoPool(repo: string, zecAmount: number) {
  const pool = getOrCreateRepoPool(repo);
  pool.poolBalanceZec += zecAmount;
  pool.poolBalanceUsd += zecAmount * ZEC_PRICE_USD;
  notify(`Successfully funded ${zecAmount} ZEC into pool for ${repo}!`);
}
