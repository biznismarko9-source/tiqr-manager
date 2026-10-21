// 2.113.1 - muted senders, the screen's side (the rules are in
// commands/mail.rs, "MUTED SENDERS").
//
// marko: "chcem aj funkcie hide those mails ak vela spamuju a na nejaky cas si
// vies vypnut aby si ich neprijmal teda aj hej ale skryl ich". A mute hides a
// sender's mail - it still arrives - for a while or until unmuted, and never
// hides an order, a sale or a payout.
import type { MailMute } from "./types";

/** How long a mute can last, as the menu offers it. `null`: until unmuted. */
export const MUTE_SPANS: { days: number | null; label: string }[] = [
  { days: 1, label: "1 day" },
  { days: 7, label: "7 days" },
  { days: 30, label: "30 days" },
  { days: null, label: "Always" },
];

/** Mailboxes everyone has - the same list as the backend's. Muting a whole
 *  one would hide every person who writes from it, so only the address can
 *  be muted there. */
const SHARED_MAIL_DOMAINS = new Set([
  "gmail.com", "googlemail.com", "outlook.com", "hotmail.com", "live.com", "msn.com", "yahoo.com", "icloud.com",
  "me.com", "aol.com", "proton.me", "protonmail.com", "gmx.com", "gmx.de", "gmx.net", "web.de", "seznam.cz",
  "email.cz", "centrum.cz", "centrum.sk", "azet.sk", "zoznam.sk", "post.sk", "atlas.sk", "pobox.sk", "wp.pl",
  "o2.pl", "onet.pl", "interia.pl", "freemail.hu", "yandex.com", "mail.ru",
]);

export const domainOf = (address: string): string => {
  const at = address.lastIndexOf("@");
  return at < 0 ? "" : address.slice(at + 1).trim().toLowerCase();
};

/** Whether everyone at `domain` may be muted at once. */
export const canMuteDomain = (domain: string): boolean =>
  domain.includes(".") && !SHARED_MAIL_DOMAINS.has(domain.toLowerCase());

export type MuteKind = "sender" | "subject";

/** 2.115.0: what a mute hides by - a mute from before has no kind and is a
 *  sender. */
export const muteKind = (m: MailMute): MuteKind => (m.kind === "subject" ? "subject" : "sender");

/** The mute that covers an address - its own first, then its domain's (a
 *  domain covers its subdomains, as in the backend). */
export function muteFor(address: string, mutes: MailMute[]): MailMute | null {
  const a = address.trim().toLowerCase();
  const senders = mutes.filter((m) => muteKind(m) === "sender");
  const own = senders.find((m) => m.who === a);
  if (own) return own;
  return senders.find((m) => !m.who.includes("@") && (a.endsWith(`@${m.who}`) || a.endsWith(`.${m.who}`))) ?? null;
}

/** The same folding as the backend's `fold_words` (commands/mail_facts.rs),
 *  letter for letter: lower case, accents folded, words only, one space
 *  between them. */
const FOLDED: Record<string, string> = { ß: "ss", "'": "", "’": "", "&": " and " };
for (const [letters, to] of [
  ["áäàâãåÁÄÀÂ", "a"],
  ["čçČÇ", "c"],
  ["ďĎ", "d"],
  ["éěèêëÉĚÈ", "e"],
  ["íìîïÍ", "i"],
  ["ľĺĽĹ", "l"],
  ["ňñŇÑ", "n"],
  ["óôöòõÓÔÖ", "o"],
  ["ŕřŔŘ", "r"],
  ["šŠ", "s"],
  ["ťŤ", "t"],
  ["úůüùûÚŮÜ", "u"],
  ["ýÿÝ", "y"],
  ["žŽ", "z"],
] as const) {
  for (const ch of letters) FOLDED[ch] = to;
}

export function foldWords(text: string): string {
  let out = "";
  for (const ch of text) {
    const to = FOLDED[ch];
    if (to !== undefined) out += to;
    else if (/[\p{L}\p{N}]/u.test(ch)) out += ch.toLowerCase();
    else out += " ";
  }
  return out.split(/\s+/).filter(Boolean).join(" ");
}

/** 2.115.0: the subject mute that covers a subject - its words, whole and in
 *  that order, as in the backend. */
export function subjectMuteFor(subject: string, mutes: MailMute[]): MailMute | null {
  const hay = ` ${foldWords(subject)} `;
  return (
    mutes.find((m) => {
      if (muteKind(m) !== "subject") return false;
      const words = foldWords(m.who);
      return words.length > 0 && hay.includes(` ${words} `);
    }) ?? null
  );
}

/** 2.115.0: the words the Mute menu offers for a subject - without "Re:" or
 *  "Fwd:" in front and without the order number, count or date at the end,
 *  which change from one mail to the next. Edited before it is used. */
export function subjectSuggestion(subject: string): string {
  let s = subject.replace(/\s+/g, " ").trim();
  for (;;) {
    const next = s.replace(/^(re|fw|fwd|aw|wg|tr|sv|vs|odp|odpoveď|odpoved)\s*(\[\d+\])?\s*:\s*/i, "");
    if (next === s) break;
    s = next;
  }
  for (;;) {
    const next = s
      // "#45", "No. 7", "č. 123": a number marked as one, however short...
      .replace(/[\s:|,.\-–—]*(#|no\.|nr\.|č\.)\s*[a-z0-9][a-z0-9\-/.]*$/i, "")
      // ...or a long one on its own: an order code, a date.
      .replace(/[\s:|,.\-–—]*[a-z]{0,3}-?\d[\d\-/.]{3,}$/i, "")
      .trim();
    if (next === s) break;
    s = next;
  }
  s = s.replace(/[\s\-–—:|,.!?]+$/, "").trim();
  if (s.length > 60) {
    const cut = s.slice(0, 60);
    const space = cut.lastIndexOf(" ");
    s = (space > 20 ? cut.slice(0, space) : cut).trim();
  }
  return s || subject.trim().slice(0, 60);
}

/** Still hiding new mail. An ended mute keeps hiding only what came before. */
export const muteActive = (m: MailMute, now = Date.now()): boolean =>
  !m.until || new Date(m.until).getTime() > now;

/** "until you unmute", "for 6 more days", "until tomorrow", "ended 21 Oct". */
export function muteState(m: MailMute, now = Date.now()): string {
  if (!m.until) return "until you unmute";
  const end = new Date(m.until).getTime();
  if (!Number.isFinite(end)) return "until you unmute";
  if (end <= now) {
    return `ended ${new Date(end).toLocaleDateString([], { day: "numeric", month: "short" })} - new mail shows again`;
  }
  const days = Math.ceil((end - now) / 86_400_000);
  if (days <= 1) return "until tomorrow";
  return `for ${days} more days`;
}
