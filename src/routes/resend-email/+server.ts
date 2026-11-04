import { json } from '@sveltejs/kit';
import { ORGANIZER_API_TOKEN, RESEND_API_KEY, RESEND_FROM_EMAIL } from '$app/env/private';
import { organizerAuthorized } from '#lib/server/organizer-auth.ts';
import { dispatchClaimEmails, parseRecipients } from '#lib/server/claim-email.ts';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, fetch }) => {
  if (!ORGANIZER_API_TOKEN || !RESEND_API_KEY) {
    return json(
      {
        success: false,
        accepted: 0,
        error:
          'Email sending is not configured. Set RESEND_API_KEY and ORGANIZER_API_TOKEN in the frontend server environment.',
      },
      { status: 503 }
    );
  }
  if (!organizerAuthorized(request.headers.get('Authorization'), ORGANIZER_API_TOKEN)) {
    return json(
      {
        success: false,
        accepted: 0,
        error: 'Organizer authentication is required to send claim emails.',
      },
      { status: 401 }
    );
  }
  let body;
  let recipients;
  try {
    body = await request.json();
    recipients = parseRecipients(body?.recipients);
  } catch (error) {
    return json(
      {
        success: false,
        error:
          error instanceof SyntaxError
            ? 'Invalid JSON request.'
            : error instanceof Error
              ? error.message
              : 'Invalid request.',
      },
      { status: 400 }
    );
  }
  const roomName =
    typeof body.roomName === 'string' ? body.roomName.trim().slice(0, 80) : 'Zool Room';
  const result = await dispatchClaimEmails(
    recipients,
    {
      apiKey: RESEND_API_KEY,
      fromEmail: RESEND_FROM_EMAIL,
      roomName: roomName || 'Zool Room',
    },
    fetch
  );
  return json(result, {
    status: result.accepted === 0 ? 502 : 200,
    headers: { 'Cache-Control': 'no-store' },
  });
};
