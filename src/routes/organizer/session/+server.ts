import { json } from '@sveltejs/kit';
import { ORGANIZER_API_TOKEN } from '$app/env/private';
import { organizerAuthorized } from '#lib/server/organizer-auth.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = ({ request }) => {
  const allowed = organizerAuthorized(request.headers.get('Authorization'), ORGANIZER_API_TOKEN);
  return json(
    { authenticated: allowed },
    {
      status: allowed ? 200 : 401,
      headers: { 'Cache-Control': 'no-store' },
    }
  );
};
