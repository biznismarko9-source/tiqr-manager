// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { canMuteDomain, domainOf, muteActive, muteFor, muteState } from "./mailMute.ts";

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
