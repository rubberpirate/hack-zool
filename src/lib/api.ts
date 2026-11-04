/**
 * Z-Split Frontend API Client
 * Connects the Svelte frontend to the Axum backend.
 */

import { randomUuid } from './uuid.ts';

const DEFAULT_AUTH_TOKEN = import.meta.env?.DEV
  ? 'zsplit_local_dev_organizer_api_token_32chars'
  : '';

export function getAuthToken(): string {
  if (typeof localStorage !== 'undefined') {
    return localStorage.getItem('zsplit-auth-token') || DEFAULT_AUTH_TOKEN;
  }
  return DEFAULT_AUTH_TOKEN;
}

export function setAuthToken(token: string) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('zsplit-auth-token', token);
  }
}

export interface BackendRoom {
  id: string;
  network: string;
  status: 'awaiting_funds' | 'ready' | 'paused' | 'closed';
  amounts_zat: number[];
  fee_budget_per_ticket_zat: number;
  confirmed_zat: number;
  reserved_zat: number;
  claims_issued: boolean;
  expires_at: string;
}

export interface DepositResponse {
  room_id: string;
  network: string;
  address: string;
  birthday_height: number;
  confirmed_zat: number;
  spendable_zat: number;
  required_zat?: number;
  is_ready?: boolean;
  scanned_height: number;
  chain_height: number;
}

export interface RoomSummaryResponse {
  room_id: string;
  confirmed_zat: number;
  reserved_zat: number;
  spendable_zat: number;
  reserved_count: number;
  pending_count: number;
  funded_confirmed_count: number;
  expired_count: number;
  sweep_reported_count: number;
  manual_review_count: number;
  sweep_reports_verified: boolean;
}

export interface IssuedClaimTicket {
  id: string;
  token: string;
  amount_zat: number;
}

export interface ClaimMetadata {
  claim_id: string;
  network: string;
  amount_zat: number;
  status:
    | 'reserved'
    | 'funding_queued'
    | 'preparing'
    | 'funding_broadcast'
    | 'funded_unconfirmed'
    | 'funded_confirmed'
    | 'expired'
    | 'cancelled'
    | 'ambiguous_broadcast'
    | 'reconciling'
    | 'reorged'
    | 'manual_review';
  expires_at: string;
  txid?: string | null;
  confirmations?: number;
  sweep_reported?: boolean;
}

export interface RedeemResponse {
  claim_id: string;
  status: string;
  outbox_id?: string;
  txid?: string;
}

export async function createBackendRoom(
  amountsZat: number[],
  expiresAt: string,
  feeReserveZat = 10_000
): Promise<BackendRoom> {
  const res = await fetch('/api/v1/rooms', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getAuthToken()}`,
    },
    body: JSON.stringify({
      network: 'testnet',
      amounts_zat: amountsZat,
      expires_at: expiresAt,
      fee_reserve_zat: feeReserveZat,
    }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to create room: ${res.statusText}`);
  }
  return res.json();
}

export async function getBackendDeposit(roomId: string): Promise<DepositResponse> {
  const res = await fetch(`/api/v1/rooms/${roomId}/deposit`, {
    headers: {
      Authorization: `Bearer ${getAuthToken()}`,
    },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to get deposit info: ${res.statusText}`);
  }
  return res.json();
}

export async function devConfirmBackendDeposit(roomId: string): Promise<DepositResponse> {
  const res = await fetch(`/api/v1/rooms/${roomId}/dev-confirm`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${getAuthToken()}`,
    },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to confirm deposit: ${res.statusText}`);
  }
  return res.json();
}

export async function getBackendSummary(roomId: string): Promise<RoomSummaryResponse> {
  const res = await fetch(`/api/v1/rooms/${roomId}/summary`, {
    headers: {
      Authorization: `Bearer ${getAuthToken()}`,
    },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to get room summary: ${res.statusText}`);
  }
  return res.json();
}

export async function issueBackendClaims(roomId: string): Promise<IssuedClaimTicket[]> {
  const res = await fetch(`/api/v1/rooms/${roomId}/claims`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${getAuthToken()}`,
    },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to issue claims: ${res.statusText}`);
  }
  const data = await res.json();
  const tickets = Array.isArray(data) ? data : data.tickets || [];
  return tickets.map((ticket: IssuedClaimTicket & { claim_id?: string }) => ({
    ...ticket,
    id: ticket.claim_id || ticket.id,
  }));
}

export async function getBackendClaim(
  claimId: string,
  token: string,
  signal?: AbortSignal
): Promise<ClaimMetadata> {
  const res = await fetch(`/api/v1/claims/${claimId}`, {
    headers: { Authorization: `Bearer ${token}` },
    cache: 'no-store',
    signal,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to get claim: ${res.statusText}`);
  }
  return res.json();
}

export async function getBackendClaimStatus(
  claimId: string,
  token: string,
  signal?: AbortSignal
): Promise<ClaimMetadata> {
  const res = await fetch(`/api/v1/claims/${claimId}/status`, {
    headers: { Authorization: `Bearer ${token}` },
    cache: 'no-store',
    signal,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to get claim status: ${res.statusText}`);
  }
  return res.json();
}

export async function redeemBackendClaim(
  claimId: string,
  token: string,
  receiver: string,
  idempotencyKey = randomUuid(),
  walletPersisted = false
): Promise<RedeemResponse> {
  const res = await fetch(`/api/v1/claims/${claimId}/redeem`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({
      ephemeral_receiver: receiver,
      idempotency_key: idempotencyKey,
      client_network: 'testnet',
      wallet_persisted: walletPersisted,
    }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || `Failed to redeem claim: ${res.statusText}`);
  }
  return res.json();
}

export async function reportBackendSweep(claimId: string, token: string): Promise<void> {
  const res = await fetch(`/api/v1/claims/${claimId}/sweep-report`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!res.ok) throw new Error('Could not record the unverified sweep report.');
}

export interface FrostSetupResponse {
  min_signers: number;
  max_signers: number;
  group_public_key: string;
  public_key_package: string;
  key_packages: string[];
}

export interface FrostSignResponse {
  signature_hex: string;
  verified: boolean;
}

export async function setupBackendFrost(): Promise<FrostSetupResponse> {
  const res = await fetch('/api/v1/frost/setup', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || 'Failed to setup FROST threshold treasury');
  }
  return res.json();
}

export async function signBackendFrost(
  groupPublicKey: string,
  message: string,
  keyPackages: string[],
  publicKeyPackage: string
): Promise<FrostSignResponse> {
  const res = await fetch('/api/v1/frost/sign', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      group_public_key: groupPublicKey,
      message,
      key_packages: keyPackages,
      public_key_package: publicKeyPackage,
    }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error(err.message || 'Failed to sign with FROST');
  }
  return res.json();
}
