import QRCode from 'qrcode';
import { claimAmount, daysLeft, type Room } from './model';
// A small vector PDF keeps exports crisp without shipping a PDF framework.
export function downloadClaimsPDF(room: Room, origin: string) {
  const objects: string[] = [];
  const add = (s: string) => {
    objects.push(s);
    return objects.length;
  };
  const catalog = add('');
  const pages = add('');
  const font = add('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>');
  const pageRefs: number[] = [];
  const text = (value: string, x: number, y: number, size: number) =>
    `BT /F1 ${size} Tf ${x} ${y} Td (${value.replace(/[^\x20-\x7E]/g, '').replace(/[\\()]/g, '\\$&')}) Tj ET\n`;
  for (let start = 1; start <= room.count; start += 12) {
    let stream = text('Z-SPLIT / ONE-TIME CLAIMS', 32, 805, 20) + text(room.name, 32, 782, 11);
    for (let i = start; i < Math.min(start + 12, room.count + 1); i++) {
      const col = (i - start) % 3,
        row = Math.floor((i - start) / 3),
        x = 32 + col * 180,
        y = 597 - row * 181;
      stream +=
        `0 0 0 RG ${x} ${y} 169 172 re S\n` +
        text(`No. ${String(i).padStart(2, '0')}`, x + 12, y + 151, 12);
      if (room.claimed.includes(i)) stream += text('CLAIMED', x + 39, y + 78, 17);
      else if (room.closed || daysLeft(room) === 0)
        stream += text(room.closed ? 'CLOSED' : 'EXPIRED', x + 39, y + 78, 17);
      else {
        const qr = QRCode.create(`${origin}/claim/${room.id}-${i}`, {
          errorCorrectionLevel: 'M',
        }).modules;
        const unit = 108 / qr.size;
        for (let r = 0; r < qr.size; r++)
          for (let c = 0; c < qr.size; c++)
            if (qr.get(r, c))
              stream += `${(x + 30 + c * unit).toFixed(3)} ${(y + 34 + (qr.size - r - 1) * unit).toFixed(3)} ${(unit + 0.01).toFixed(3)} ${(unit + 0.01).toFixed(3)} re f\n`;
      }
      stream += text(`${claimAmount(room, i).toFixed(2)} ZEC / works once`, x + 12, y + 14, 9);
    }
    stream += text(
      'Keep each link private. Anyone holding an unused link can claim it.',
      32,
      28,
      9
    );
    const content = add(`<< /Length ${stream.length} >>\nstream\n${stream}endstream`);
    pageRefs.push(
      add(
        `<< /Type /Page /Parent ${pages} 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 ${font} 0 R >> >> /Contents ${content} 0 R >>`
      )
    );
  }
  objects[catalog - 1] = `<< /Type /Catalog /Pages ${pages} 0 R >>`;
  objects[pages - 1] =
    `<< /Type /Pages /Kids [${pageRefs.map((p) => `${p} 0 R`).join(' ')}] /Count ${pageRefs.length} >>`;
  let doc = '%PDF-1.4\n';
  const offsets = [0];
  objects.forEach((object, i) => {
    offsets.push(doc.length);
    doc += `${i + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = doc.length;
  doc +=
    `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n` +
    offsets
      .slice(1)
      .map((o) => `${String(o).padStart(10, '0')} 00000 n \n`)
      .join('') +
    `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF`;
  const url = URL.createObjectURL(new Blob([doc], { type: 'application/pdf' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = `z-split-${room.id}-claims.pdf`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
