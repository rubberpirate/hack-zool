import { demoRoom, validRoom, daysLeft, claimAmount, type Room } from './model';
import {
  createBackendRoom,
  getBackendDeposit,
  getBackendSummary,
  issueBackendClaims,
  type BackendRoom,
  type DepositResponse,
  type IssuedClaimTicket,
  type RoomSummaryResponse,
} from './api';

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
  // Native Backend state
  activeMode: 'testnet' as 'testnet' | 'demo',
  backendRoom: null as BackendRoom | null,
  depositAddress: '',
  depositRequiredZat: 0,
  depositConfirmedZat: 0,
  issuedTickets: [] as IssuedClaimTicket[],
  summary: null as RoomSummaryResponse | null,
  frostSetup: null as {
    min_signers: number;
    max_signers: number;
    group_public_key: string;
    public_key_package: string;
    key_packages: string[];
    authorized: boolean;
    signature_hex?: string;
  } | null,
});

export function loadRoom() {
  try {
    const stored = JSON.parse(localStorage.getItem('zool-room') || localStorage.getItem('z-split-room') || 'null');
    if (validRoom(stored)) app.room = stored;
    const sample = JSON.parse(sessionStorage.getItem('zool-sample') || sessionStorage.getItem('z-split-sample') || 'null');
    if (validRoom(sample) && sample.id === 'sample' && daysLeft(sample) > 0) app.sample = sample;

    const storedBackend = JSON.parse(localStorage.getItem('zool-backend-room') || localStorage.getItem('z-split-backend-room') || 'null');
    if (storedBackend) app.backendRoom = storedBackend;

    const storedDeposit = localStorage.getItem('zool-deposit-addr') || localStorage.getItem('z-split-deposit-addr') || '';
    if (storedDeposit) app.depositAddress = storedDeposit;

    const storedTickets = JSON.parse(localStorage.getItem('zool-issued-tickets') || localStorage.getItem('z-split-issued-tickets') || '[]');
    if (Array.isArray(storedTickets)) app.issuedTickets = storedTickets;
  } catch {
    /* Fresh state if local storage is unavailable. */
  }
  app.ready = true;
}

export function saveRoom() {
  try {
    localStorage.setItem('zool-room', JSON.stringify(app.room));
    localStorage.setItem('z-split-room', JSON.stringify(app.room));
    if (app.backendRoom) {
      localStorage.setItem('zool-backend-room', JSON.stringify(app.backendRoom));
      localStorage.setItem('z-split-backend-room', JSON.stringify(app.backendRoom));
    }
    if (app.depositAddress) {
      localStorage.setItem('zool-deposit-addr', app.depositAddress);
      localStorage.setItem('z-split-deposit-addr', app.depositAddress);
    }
    if (app.issuedTickets.length > 0) {
      localStorage.setItem('zool-issued-tickets', JSON.stringify(app.issuedTickets));
      localStorage.setItem('z-split-issued-tickets', JSON.stringify(app.issuedTickets));
    }
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

export async function createRealRoom(
  values: Pick<Room, 'name' | 'count' | 'amount' | 'expiry' | 'giftCards' | 'split'>,
  customAmounts?: number[]
) {
  // Compute per-claim payout amounts respecting equal or tiered prizes (1st, 2nd, 3rd... price)
  const amounts = customAmounts && customAmounts.length === values.count
    ? customAmounts
    : Array.from({ length: values.count }, (_, i) => {
        const singleZec = claimAmount({ ...app.room, ...values }, i + 1);
        return Math.round(singleZec * 100_000_000);
      });
  const expiresAt = new Date(Date.now() + values.expiry * 86_400_000).toISOString();

  try {
    const backendRoom = await createBackendRoom(amounts, expiresAt, 10_000);
    const depositInfo = await getBackendDeposit(backendRoom.id);

    app.backendRoom = backendRoom;
    app.depositAddress = depositInfo.address;
    app.depositRequiredZat = depositInfo.required_zat || (amounts.reduce((a, b) => a + b, 0) + (backendRoom.fee_budget_per_ticket_zat * amounts.length));
    app.depositConfirmedZat = depositInfo.confirmed_zat;
    app.issuedTickets = [];

    // Also mirror to app.room for compatibility with UI components
    app.room = {
      ...values,
      id: backendRoom.id,
      claimed: [],
      gifts: [],
      closed: false,
      funded: depositInfo.is_ready || depositInfo.confirmed_zat >= app.depositRequiredZat,
      createdAt: Date.now(),
    };
    saveRoom();
    return backendRoom;
  } catch (err: any) {
    // If backend isn't available or fails, notify and fallback to demo mode
    console.error('Backend room creation failed, falling back to local mode:', err);
    createRoom(values);
    notify(`Backend connection warning: ${err.message || 'Offline'}. Using local demo.`);
    return null;
  }
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

export async function checkDepositStatus(): Promise<DepositResponse | null> {
  if (!app.backendRoom?.id) return null;
  try {
    const deposit = await getBackendDeposit(app.backendRoom.id);
    app.depositConfirmedZat = deposit.confirmed_zat;
    if (deposit.required_zat) {
      app.depositRequiredZat = deposit.required_zat;
    }
    if (deposit.is_ready || (deposit.confirmed_zat >= app.depositRequiredZat && app.depositRequiredZat > 0)) {
      app.room.funded = true;
    }
    saveRoom();
    return deposit;
  } catch (err) {
    console.error('Failed to check deposit:', err);
    return null;
  }
}

export async function mintClaims() {
  if (!app.backendRoom?.id) return [];
  try {
    const tickets = await issueBackendClaims(app.backendRoom.id);
    app.issuedTickets = tickets;
    saveRoom();
    return tickets;
  } catch (err: any) {
    console.error('Failed to issue claims from backend:', err);
    throw err;
  }
}

export function claimLink(number: number) {
  if (app.issuedTickets && app.issuedTickets[number - 1]) {
    const ticket = app.issuedTickets[number - 1];
    return `${location.origin}/claim/${ticket.id}#t=${ticket.token}`;
  }
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
