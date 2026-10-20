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

/** The mute that covers an address - its own first, then its domain's (a
 *  domain covers its subdomains, as in the backend). */
export function muteFor(address: string, mutes: MailMute[]): MailMute | null {
  const a = address.trim().toLowerCase();
  const own = mutes.find((m) => m.who === a);
  if (own) return own;
  return mutes.find((m) => !m.who.includes("@") && (a.endsWith(`@${m.who}`) || a.endsWith(`.${m.who}`))) ?? null;
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
