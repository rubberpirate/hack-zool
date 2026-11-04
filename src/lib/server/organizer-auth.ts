import { createHash, timingSafeEqual } from 'node:crypto';

export function organizerAuthorized(authorization: string | null, secret: string): boolean {
  if (!secret) return false;
  const digest = (value: string) => createHash('sha256').update(value).digest();
  return timingSafeEqual(digest(authorization || ''), digest(`Bearer ${secret}`));
}
