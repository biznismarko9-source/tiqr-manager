// 2.113.0 - the small, pure pieces of the mail reader (components/MailReader.tsx).
//
// marko: "linky ktore su v maily nech su klikatelne a ukazuju sa tak ako v
// maily", "aby som to vedel aj jednym klikom skopirovat", "OTP kody nech sa
// skopiruju na klik" and "nech len nevypisuje mail ale aj ho pekne zobrazi".
// And over all of it: "e-mailovy obsah a prilohy treba spracuvat ako
// nedoveryhodny vstup". So nothing here builds markup from an email's text:
// plain text is cut into pieces the screen draws as React elements, and the
// one piece of markup there is - the page the designed email is shown in -
// wraps HTML that Rust already cleaned, and forbids it everything.

/** One piece of a line of plain text. */
export type TextPiece =
  | { kind: "text"; text: string }
  | { kind: "link"; text: string; href: string }
  | { kind: "email"; text: string; href: string }
  | { kind: "code"; text: string };

/** A run of lines: an ordinary paragraph, or text quoted from an earlier
 *  message (`> ` lines), which reads quieter. */
export interface TextBlock {
  kind: "para" | "quote";
  lines: string[];
}

const URL_START = String.raw`\bhttps?:\/\/[^\s<>"']+|\bwww\.[^\s<>"']+`;
const EMAIL = String.raw`\b[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b`;

const count = (s: string, c: string) => s.split(c).length - 1;

/** A link found in running text, without the full stop or bracket that ends
 *  the sentence around it. */
function trimUrl(raw: string): string {
  let url = raw;
  for (;;) {
    const last = url.slice(-1);
    if (last && ".,;:!?'\"*".includes(last)) url = url.slice(0, -1);
    else if (last === ")" && count(url, "(") < count(url, ")")) url = url.slice(0, -1);
    else if (last === "]" && count(url, "[") < count(url, "]")) url = url.slice(0, -1);
    else return url;
  }
}

const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** One line of plain text cut into words, links, addresses and - when the
 *  message has one - its one-time code. */
export function linkify(text: string, otp?: string | null): TextPiece[] {
  const code = otp && otp.trim().length >= 4 ? otp.trim() : null;
  const pattern = new RegExp(
    `(${URL_START})|(${EMAIL})${code ? `|((?<![\\w-])${escapeRe(code)}(?![\\w-]))` : ""}`,
    "g",
  );
  const out: TextPiece[] = [];
  let at = 0;
  const pushText = (s: string) => {
    if (!s) return;
    const prev = out[out.length - 1];
    if (prev?.kind === "text") prev.text += s;
    else out.push({ kind: "text", text: s });
  };
  for (const m of text.matchAll(pattern)) {
    const start = m.index ?? 0;
    pushText(text.slice(at, start));
    if (m[1]) {
      const url = trimUrl(m[1]);
      out.push({ kind: "link", text: url, href: url.toLowerCase().startsWith("www.") ? `https://${url}` : url });
      pushText(m[1].slice(url.length));
    } else if (m[2]) {
      out.push({ kind: "email", text: m[2], href: `mailto:${m[2]}` });
    } else if (m[3]) {
      out.push({ kind: "code", text: m[3] });
    }
    at = start + m[0].length;
  }
  pushText(text.slice(at));
  return out;
}

/** Plain text in paragraphs and quotes, with runs of blank lines folded into
 *  one break - the way a mail program shows it, not a wall of text. */
export function textBlocks(text: string): TextBlock[] {
  const blocks: TextBlock[] = [];
  let current: TextBlock | null = null;
  for (const raw of text.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.replace(/\s+$/, "");
    const quoted = /^\s*>/.test(line);
    if (!quoted && line.trim() === "") {
      current = null;
      continue;
    }
    const kind = quoted ? "quote" : "para";
    const content = quoted ? line.replace(/^\s*>\s?/, "") : line;
    if (!current || current.kind !== kind) {
      current = { kind, lines: [] };
      blocks.push(current);
    }
    current.lines.push(content);
  }
  // A quote ends with the blank lines it was written with - not shown.
  for (const b of blocks) {
    while (b.lines.length > 1 && b.lines[b.lines.length - 1].trim() === "") b.lines.pop();
  }
  return blocks;
}

/** The site a link goes to (`viagogo.com`), or null for one that is not a
 *  web address. */
export function hostOf(href: string): string | null {
  try {
    const u = new URL(href);
    if (u.protocol !== "http:" && u.protocol !== "https:") return null;
    return u.hostname.toLowerCase() || null;
  } catch {
    return null;
  }
}

const bare = (host: string) => host.replace(/^www\./, "");

/** When a link's TEXT names one web address and the link opens another, the
 *  address it shows - a phishing mail's favourite trick ("paypal.com" that
 *  opens somewhere else). Null when they agree, or when the text is not an
 *  address at all ("Track your order"). A subdomain of the same site is the
 *  same site. */
export function shownElsewhere(text: string, href: string): string | null {
  const m = text.trim().match(/^(?:https?:\/\/)?((?:[a-z0-9-]+\.)+[a-z]{2,})(?:[/:?#]\S*)?$/i);
  if (!m) return null;
  const real = hostOf(href);
  if (!real) return null;
  const shown = bare(m[1].toLowerCase());
  const goes = bare(real);
  if (shown === goes || goes.endsWith(`.${shown}`) || shown.endsWith(`.${goes}`)) return null;
  return m[1].toLowerCase();
}

/** A file size the way a mail program writes it. */
export function fileSize(bytes: number): string {
  if (!(bytes > 0)) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** What an attachment's bytes are, for a preview - by the bytes, never by
 *  the name. Only what the backend lets through can arrive here: a picture,
 *  a PDF or text. */
export function previewType(bytes: Uint8Array): { kind: "image" | "pdf" | "text"; mime: string } {
  const at = (i: number, ...b: number[]) => b.every((x, j) => bytes[i + j] === x);
  if (at(0, 0x25, 0x50, 0x44, 0x46)) return { kind: "pdf", mime: "application/pdf" };
  if (at(0, 0x89, 0x50, 0x4e, 0x47)) return { kind: "image", mime: "image/png" };
  if (at(0, 0xff, 0xd8, 0xff)) return { kind: "image", mime: "image/jpeg" };
  if (at(0, 0x47, 0x49, 0x46, 0x38)) return { kind: "image", mime: "image/gif" };
  if (at(0, 0x52, 0x49, 0x46, 0x46) && at(8, 0x57, 0x45, 0x42, 0x50)) return { kind: "image", mime: "image/webp" };
  return { kind: "text", mime: "text/plain" };
}

/* ---------------------------------------------------------------------------
   The page a designed email is shown in.

   The HTML arrives CLEANED (commands/mail_view.rs: no script, no handler, no
   form, no frame, links http/https/mailto only, pictures already copied into
   TIQR and inside it as data). This page is the second wall: it is shown in
   an iframe that may run no script at all (`sandbox` without allow-scripts),
   and its policy lets it load nothing - no picture, font or style from the
   web, so not even a tracking pixel that slipped through can phone home.
   `<base target="_blank">` makes every link a new window, which the sandbox
   refuses - so even if the reader's own click handling failed, a link would
   do nothing rather than load a web page into TIQR.
   --------------------------------------------------------------------------- */

export const READER_CSP =
  "default-src 'none'; img-src data:; style-src 'unsafe-inline'; font-src data:; form-action 'none'";

/* The defaults a mail program gives a message. The email's own styles come
   after these and win, so a designed newsletter looks the way it was made;
   these only stop a careless one from spilling off the screen. A picture
   with a height of its own keeps it - a 600 x 1 spacer forced to "auto"
   would become a 600 x 600 hole - and is scaled inside its box instead. */
const READER_STYLE = `
html,body{margin:0;padding:0;background:#ffffff;}
body{padding:18px 20px;color:#1f2937;font:14px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;overflow-wrap:break-word;}
img{max-width:100%;}
img:not([height]){height:auto;}
img[height]{object-fit:contain;}
table{max-width:100%;}
pre{white-space:pre-wrap;}
a{color:#6d28d9;}
blockquote{margin:8px 0 8px 4px;padding-left:12px;border-left:3px solid #e5e7eb;color:#4b5563;}
.tiqr-otp{background:#ede9fe;border-radius:4px;padding:0 3px;cursor:pointer;box-shadow:0 0 0 1px #a78bfa;}
.tiqr-otp:hover{background:#ddd6fe;}
.tiqr-otp.tiqr-copied{background:#bbf7d0;box-shadow:0 0 0 1px #22c55e;}
`;

/** The whole document for the reader's iframe. */
export function readerDocument(cleanHtml: string): string {
  return (
    "<!doctype html><html><head>" +
    `<meta http-equiv="Content-Security-Policy" content="${READER_CSP}">` +
    '<meta charset="utf-8"><meta name="color-scheme" content="light"><base target="_blank">' +
    `<style>${READER_STYLE}</style></head><body>${cleanHtml}</body></html>`
  );
}

/** The languages Translate can write in - the same list as the backend's
 *  (commands/mail_view.rs::language_name). */
export const MAIL_LANGUAGES: { code: string; name: string }[] = [
  { code: "sk", name: "Slovak" },
  { code: "cs", name: "Czech" },
  { code: "en", name: "English" },
  { code: "de", name: "German" },
  { code: "hu", name: "Hungarian" },
  { code: "pl", name: "Polish" },
  { code: "fr", name: "French" },
  { code: "es", name: "Spanish" },
  { code: "it", name: "Italian" },
  { code: "nl", name: "Dutch" },
  { code: "pt", name: "Portuguese" },
  { code: "uk", name: "Ukrainian" },
];

export const languageName = (code: string | null | undefined): string =>
  MAIL_LANGUAGES.find((l) => l.code === code)?.name ?? (code ? code.toUpperCase() : "");
