import QRCode from 'qrcode';
import { createHash } from 'node:crypto';

export interface EmailPayload {
  email: string;
  number: number;
  amountZec: string;
  prizeLabel: string;
  claimUrl: string;
}
export interface EmailResult {
  email: string;
  number: number;
  success: boolean;
  id?: string;
  error: string | null;
}

export function parseRecipients(value: unknown): EmailPayload[] {
  if (!Array.isArray(value) || value.length === 0 || value.length > 100) {
    throw new Error('Provide between 1 and 100 recipients.');
  }
  return value.map((item) => {
    if (
      !item ||
      typeof item !== 'object' ||
      typeof item.email !== 'string' ||
      item.email.length > 320 ||
      !/^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/.test(item.email.trim()) ||
      !Number.isInteger(item.number) ||
      item.number < 1 ||
      item.number > 100 ||
      typeof item.amountZec !== 'string' ||
      !/^\d+(\.\d{1,8})?$/.test(item.amountZec) ||
      Number(item.amountZec) <= 0 ||
      Number(item.amountZec) > 21_000_000 ||
      typeof item.prizeLabel !== 'string' ||
      item.prizeLabel.length > 120 ||
      typeof item.claimUrl !== 'string' ||
      item.claimUrl.length > 2048
    ) {
      throw new Error(
        'Each recipient needs a valid email, claim number, amount, label and claim URL.'
      );
    }
    let url: URL;
    try {
      url = new URL(item.claimUrl);
    } catch {
      throw new Error('Invalid claim URL.');
    }
    if (
      (url.protocol !== 'https:' &&
        !(
          url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
        )) ||
      url.username ||
      url.password ||
      !url.pathname.startsWith('/claim/')
    ) {
      throw new Error('Claim URLs must use HTTPS (or localhost during development).');
    }
    return { ...item, email: item.email.trim() } as EmailPayload;
  });
}

function escapeHtml(value: string): string {
  return value.replace(
    /[&<>"']/g,
    (char) =>
      ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
      })[char]!
  );
}

export async function dispatchClaimEmails(
  recipients: EmailPayload[],
  config: { apiKey: string; fromEmail: string; roomName: string },
  fetcher: typeof fetch
) {
  const { apiKey, fromEmail, roomName } = config;
  const messages = await Promise.all(
    recipients.map(async (recipient) => {
      const qrPng = await QRCode.toBuffer(recipient.claimUrl, {
        type: 'png',
        margin: 4,
        scale: 6,
        errorCorrectionLevel: 'M',
      });
      const qrHtml =
        '<div style="text-align:center;margin:18px 0"><img src="cid:claim-qr" alt="Your claim QR code" width="240" height="240" style="display:block;margin:0 auto;max-width:100%;height:auto" /></div>';
      const html = `
<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; max-width: 560px; margin: 0 auto; padding: 24px; border: 2px solid #111; border-radius: 12px; background: #ffffff; color: #111;">
  <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 2px dashed #e2e8f0; padding-bottom: 12px; margin-bottom: 18px;">
    <span style="font-weight: 900; letter-spacing: 0.1em; font-size: 18px;">ZOOL</span>
    <span style="background: #e2e8f0; font-size: 11px; padding: 4px 8px; border-radius: 4px; font-weight: bold;">ZCASH TESTNET (NU7)</span>
  </div>
  <h2 style="margin: 0 0 12px; font-size: 22px;">Your shielded claim ticket is ready!</h2>
  <p style="font-size: 15px; color: #4a5568; line-height: 1.5; margin-bottom: 20px;">
    You were awarded funds from <strong>${escapeHtml(roomName)}</strong> on Zool.
  </p>
  <div style="background: #f7fafc; padding: 18px; border-radius: 8px; margin: 16px 0; border: 1px solid #cbd5e0; text-align: center;">
    <p style="margin: 0; font-size: 12px; font-weight: 800; color: #718096; letter-spacing: 0.05em; text-transform: uppercase;">Your Allocated Prize</p>
    <p style="margin: 6px 0 0; font-size: 28px; font-weight: 900; color: #1a202c;">${escapeHtml(recipient.prizeLabel)}</p>
    <p style="margin: 4px 0 0; font-size: 22px; font-weight: 800; color: #2b6cb0;">${escapeHtml(recipient.amountZec)} TAZ</p>
  </div>
  ${qrHtml}
  <p style="font-size: 14px; color: #4a5568; line-height: 1.5;">
    Tap below to open your unique one-time claim card and sweep the shielded funds directly to your Zingo or Zodl wallet address.
  </p>
  <div style="text-align: center; margin: 26px 0;">
    <a href="${escapeHtml(recipient.claimUrl)}" style="background: #111; color: #fff; text-decoration: none; padding: 14px 28px; border-radius: 8px; font-weight: 800; font-size: 16px; display: inline-block;">
      Claim ${escapeHtml(recipient.amountZec)} TAZ Now →
    </a>
  </div>
  <p style="font-size: 12px; color: #718096; word-break: break-all; text-align: center;">
    Direct link: <a href="${escapeHtml(recipient.claimUrl)}" style="color: #3182ce;">${escapeHtml(recipient.claimUrl)}</a>
  </p>
  <hr style="border: none; border-top: 1px solid #edf2f7; margin: 24px 0 16px;" />
  <p style="font-size: 11px; color: #a0aec0; margin: 0; line-height: 1.4;">
    🔒 <strong>Privacy Guard</strong>: This claim link contains your private 256-bit bearer token in the URL fragment (<code>#t=...</code>). Anyone with this link can redeem it; keep it private. Zool and email providers handle the recipient email address and claim link.
  </p>
</div>
`;
      return {
        from: fromEmail,
        to: [recipient.email],
        subject: `${recipient.prizeLabel}: ${recipient.amountZec} TAZ Shielded Claim from ${roomName}`,
        html,
        attachments: [
          {
            filename: 'claim-qr.png',
            content: qrPng.toString('base64'),
            content_type: 'image/png',
            content_id: 'claim-qr',
          },
        ],
      };
    })
  );
  const results: EmailResult[] = [];
  for (const [index, message] of messages.entries()) {
    // Resend's batch endpoint cannot carry inline attachments; stay under 2 requests/second.
    if (index > 0) await new Promise((resolve) => setTimeout(resolve, 600));
    const recipient = recipients[index];
    const payload = JSON.stringify(message);
    let response: Response;
    try {
      response = await fetcher('https://api.resend.com/emails', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
          'Idempotency-Key': `zool-claim-${createHash('sha256').update(payload).digest('hex')}`,
        },
        body: payload,
        signal: AbortSignal.timeout(30_000),
      });
    } catch {
      results.push({
        email: recipient.email,
        number: recipient.number,
        success: false,
        error:
          'Could not confirm whether Resend accepted this email. Check the Resend logs before retrying.',
      });
      continue;
    }
    const data = await response.json().catch(() => null);
    const success = response.ok && typeof data?.id === 'string' && data.id.length > 0;
    let error: string | null = null;
    if (!success) {
      error =
        typeof data?.message === 'string'
          ? data.message
          : response.ok
            ? 'Resend did not return a message ID. Check its logs before retrying.'
            : `Resend rejected the email (HTTP ${response.status}).`;
      if (response.status === 403 && fromEmail.includes('@resend.dev')) {
        error +=
          ' Set RESEND_FROM_EMAIL to an address on your verified Resend domain before sending to other recipients.';
      }
    }
    results.push({
      email: recipient.email,
      number: recipient.number,
      success,
      ...(success ? { id: data.id as string } : {}),
      error,
    });
  }
  const accepted = results.filter((r) => r.success).length;
  return {
    success: accepted === recipients.length,
    accepted,
    failed: recipients.length - accepted,
    ...(accepted === recipients.length ? {} : { error: results.find((r) => !r.success)?.error }),
    results,
  };
}
