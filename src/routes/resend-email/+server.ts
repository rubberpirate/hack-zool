import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

interface EmailPayload {
  email: string;
  number: number;
  amountZec: string;
  prizeLabel: string;
  claimUrl: string;
}

export const POST: RequestHandler = async ({ request }) => {
  try {
    const body = await request.json();
    const apiKey = body.apiKey?.trim() || process.env.RESEND_API_KEY || '';
    const fromEmail = body.fromEmail?.trim() || 'Zool <onboarding@resend.dev>';
    const roomName = body.roomName?.trim() || 'Zool Room';
    const recipients: EmailPayload[] = body.recipients || [];

    if (!recipients.length) {
      return json({ error: 'No recipients provided' }, { status: 400 });
    }

    const results = [];
    for (const recipient of recipients) {
      if (!recipient.email || !recipient.claimUrl) continue;

      const html = `
<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; max-width: 560px; margin: 0 auto; padding: 24px; border: 2px solid #111; border-radius: 12px; background: #ffffff; color: #111;">
  <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 2px dashed #e2e8f0; padding-bottom: 12px; margin-bottom: 18px;">
    <span style="font-weight: 900; letter-spacing: 0.1em; font-size: 18px;">Z-SPLIT</span>
    <span style="background: #e2e8f0; font-size: 11px; padding: 4px 8px; border-radius: 4px; font-weight: bold;">ZCASH TESTNET (NU7)</span>
  </div>
  <h2 style="margin: 0 0 12px; font-size: 22px;">You received a shielded payout!</h2>
  <p style="font-size: 15px; color: #4a5568; line-height: 1.5; margin-bottom: 20px;">
    You were awarded funds from <strong>${roomName}</strong> on Z-Split.
  </p>
  <div style="background: #f7fafc; padding: 18px; border-radius: 8px; margin: 16px 0; border: 1px solid #cbd5e0; text-align: center;">
    <p style="margin: 0; font-size: 12px; font-weight: 800; color: #718096; letter-spacing: 0.05em; text-transform: uppercase;">Your Allocated Prize</p>
    <p style="margin: 6px 0 0; font-size: 28px; font-weight: 900; color: #1a202c;">${recipient.prizeLabel}</p>
    <p style="margin: 4px 0 0; font-size: 22px; font-weight: 800; color: #2b6cb0;">${recipient.amountZec} TAZ</p>
  </div>
  <p style="font-size: 14px; color: #4a5568; line-height: 1.5;">
    Tap below to open your unique one-time claim card and sweep the shielded funds directly to your Zingo or Zodl wallet address.
  </p>
  <div style="text-align: center; margin: 26px 0;">
    <a href="${recipient.claimUrl}" style="background: #111; color: #fff; text-decoration: none; padding: 14px 28px; border-radius: 8px; font-weight: 800; font-size: 16px; display: inline-block;">
      Claim ${recipient.amountZec} TAZ Now →
    </a>
  </div>
  <p style="font-size: 12px; color: #718096; word-break: break-all; text-align: center;">
    Direct link: <a href="${recipient.claimUrl}" style="color: #3182ce;">${recipient.claimUrl}</a>
  </p>
  <hr style="border: none; border-top: 1px solid #edf2f7; margin: 24px 0 16px;" />
  <p style="font-size: 11px; color: #a0aec0; margin: 0; line-height: 1.4;">
    🔒 <strong>Privacy Guard</strong>: This claim link contains your private 256-bit bearer token in the URL fragment (<code>#t=...</code>). It can only be claimed once and never links to your identity on-chain.
  </p>
</div>
`;

      const resendRes = await fetch('https://api.resend.com/emails', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          from: fromEmail,
          to: [recipient.email],
          subject: `${recipient.prizeLabel}: ${recipient.amountZec} TAZ Shielded Payout from ${roomName}`,
          html,
        }),
      });

      const resData = await resendRes.json();
      results.push({
        email: recipient.email,
        number: recipient.number,
        success: resendRes.ok,
        id: resData.id,
        error: resData.message || (resendRes.ok ? null : 'Failed to deliver'),
      });
    }

    return json({
      success: true,
      delivered: results.filter((r) => r.success).length,
      failed: results.filter((r) => !r.success).length,
      results,
    });
  } catch (err: any) {
    return json({ error: err.message || 'Server error' }, { status: 500 });
  }
};
