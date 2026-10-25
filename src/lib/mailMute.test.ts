// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { canMuteDomain, domainOf, foldWords, muteActive, muteFor, muteKind, muteState, subjectMuteFor, subjectSuggestion } from "./mailMute.ts";

const now = Date.parse("2026-10-20T12:00:00Z");
const day = 86_400_000;

test("a mute covers its own address, its domain and the domain's subdomains", () => {
  const mutes = [
    { who: "news@shop.com", since: "x", until: null },
    { who: "viagogo.com", since: "x", until: new Date(now + 3 * day).toISOString() },
  ];
  assert.equal(muteFor("News@Shop.com", mutes)?.who, "news@shop.com");
  assert.equal(muteFor("deals@mail.viagogo.com", mutes)?.who, "viagogo.com");
  assert.equal(muteFor("hello@notviagogo.com", mutes), null, "a lookalike is not muted");
  assert.equal(muteFor("orders@shop.com", mutes), null, "only that sender at shop.com");
});

test("how long a mute has left reads in words", () => {
  assert.equal(muteState({ who: "a.com", since: "x", until: null }, now), "until you unmute");
  assert.equal(muteState({ who: "a.com", since: "x", until: new Date(now + 6.5 * day).toISOString() }, now), "for 7 more days");
  assert.equal(muteState({ who: "a.com", since: "x", until: new Date(now + 3_600_000).toISOString() }, now), "until tomorrow");
  assert.match(muteState({ who: "a.com", since: "x", until: new Date(now - day).toISOString() }, now), /^ended .* new mail shows again$/);
  assert.ok(muteActive({ who: "a.com", since: "x", until: null }, now));
  assert.ok(!muteActive({ who: "a.com", since: "x", until: new Date(now - 1).toISOString() }, now));
});

test("a shared mailbox domain is never muted whole", () => {
  assert.equal(domainOf("Anna@Gmail.com"), "gmail.com");
  assert.ok(!canMuteDomain("gmail.com"));
  assert.ok(!canMuteDomain("azet.sk"));
  assert.ok(canMuteDomain("email.ticketmaster.com"));
  assert.ok(!canMuteDomain("localhost"));
});

test("subject mutes: words folded as the backend folds them, whole and in order", () => {
  assert.equal(foldWords("Your WEEKLY-Digest #45"), "your weekly digest 45");
  assert.equal(foldWords("Posledná šanca: lístky na Elán & more"), "posledna sanca listky na elan and more");
  assert.equal(foldWords("Buyer's name"), "buyers name");
  const mutes = [
    { kind: "subject" as const, who: "weekly digest", since: "x", until: null },
    { who: "shop.com", since: "x", until: null },
  ];
  assert.equal(subjectMuteFor("Re: Your Weekly Digest", mutes)?.who, "weekly digest");
  assert.equal(subjectMuteFor("Weekly digests are over", mutes), null, "another word");
  assert.equal(muteKind(mutes[1]), "sender", "a mute from before 2.115.0 is a sender");
  assert.equal(muteFor("news@shop.com", mutes)?.who, "shop.com");
  assert.equal(muteFor("weekly@digest.com", [{ kind: "subject", who: "digest.com", since: "x", until: null }]), null);
});

test("the subject a mute is offered with drops what changes from one mail to the next", () => {
  assert.equal(subjectSuggestion("Re: Fwd: Your weekly digest #45"), "Your weekly digest");
  assert.equal(subjectSuggestion("Order confirmation #A-10402931"), "Order confirmation");
  assert.equal(subjectSuggestion("Payout sent - 412,50 EUR"), "Payout sent - 412,50 EUR");
  assert.equal(subjectSuggestion("Newsletter 2026-10-20"), "Newsletter");
  assert.equal(subjectSuggestion("Posledná šanca: lístky na Elán"), "Posledná šanca: lístky na Elán");
  assert.equal(subjectSuggestion("Re:"), "Re:");
  const long = "Don't miss out on these amazing weekend offers for concerts, festivals and sport events near you";
  const s = subjectSuggestion(long);
  assert.ok(s.length <= 60 && long.startsWith(s) && !s.endsWith(" "), s);
});
