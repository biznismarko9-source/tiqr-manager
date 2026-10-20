// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  fileSize,
  hostOf,
  isDownloadLink,
  linkify,
  previewType,
  READER_CSP,
  readerDocument,
  shownElsewhere,
  textBlocks,
} from "./mailText.ts";

test("links, addresses and the code are found in running text", () => {
  const pieces = linkify("Track it at https://viagogo.com/track?id=5. Questions: help@viagogo.com - code 482913.", "482913");
  assert.deepEqual(
    pieces.map((p) => p.kind),
    ["text", "link", "text", "email", "text", "code", "text"],
  );
  const link = pieces[1];
  assert.equal(link.kind === "link" && link.href, "https://viagogo.com/track?id=5", "the full stop is not part of it");
  const mail = pieces[3];
  assert.equal(mail.kind === "email" && mail.href, "mailto:help@viagogo.com");
});

test("a www address opens over https, and brackets around a link stay outside it", () => {
  const pieces = linkify("(see www.oeticket.com/info)");
  const link = pieces.find((p) => p.kind === "link");
  assert.ok(link && link.kind === "link");
  assert.equal(link.text, "www.oeticket.com/info");
  assert.equal(link.href, "https://www.oeticket.com/info");
  assert.equal(pieces[pieces.length - 1].kind === "text" && pieces[pieces.length - 1].text, ")");
  const wiki = linkify("https://en.wikipedia.org/wiki/Tour_(band)").find((p) => p.kind === "link");
  assert.equal(wiki && wiki.kind === "link" && wiki.text, "https://en.wikipedia.org/wiki/Tour_(band)", "a bracket the link opened is its own");
});

test("the code is only found as a whole word, never inside a longer number", () => {
  assert.equal(linkify("Order 14829131 and 4829 13", "4829").filter((p) => p.kind === "code").length, 1);
  assert.equal(linkify("no code here", null).length, 1);
});

test("nothing that is not a web address or an e-mail becomes a link", () => {
  for (const s of ["javascript:alert(1)", "file:///C:/x", "data:text/html,hi", "ftp://x.example"]) {
    assert.ok(!linkify(s).some((p) => p.kind === "link"), s);
  }
});

test("plain text reads in paragraphs, and quoted text is set apart", () => {
  const blocks = textBlocks("Hi Marko,\r\n\r\n\r\nYour tickets are ready.\nSee you there.\n\nOn Mon, X wrote:\n> old line\n> \n> more\n");
  assert.deepEqual(
    blocks.map((b) => [b.kind, b.lines]),
    [
      ["para", ["Hi Marko,"]],
      ["para", ["Your tickets are ready.", "See you there."]],
      ["para", ["On Mon, X wrote:"]],
      ["quote", ["old line", "", "more"]],
    ],
  );
});

test("a link whose text names another site is caught; the same site is not", () => {
  assert.equal(shownElsewhere("www.paypal.com", "https://paypa1-login.example/x"), "www.paypal.com");
  assert.equal(shownElsewhere("https://paypal.com/signin", "https://evil.example"), "paypal.com");
  assert.equal(shownElsewhere("paypal.com", "https://www.paypal.com/x"), null);
  assert.equal(shownElsewhere("viagogo.com", "https://my.viagogo.com/sales"), null, "a subdomain is the same site");
  assert.equal(shownElsewhere("Track your order", "https://anything.example"), null, "ordinary words are not an address");
  assert.equal(shownElsewhere("1.5", "https://anything.example"), null);
  assert.equal(shownElsewhere("paypal.com", "mailto:x@y.com"), null);
});

test("hostOf only knows web addresses", () => {
  assert.equal(hostOf("https://WWW.Viagogo.com/a"), "www.viagogo.com");
  assert.equal(hostOf("mailto:a@b.com"), null);
  assert.equal(hostOf("not a url"), null);
});

test("an attachment is previewed as what its bytes are", () => {
  assert.equal(previewType(new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d])).kind, "pdf");
  assert.equal(previewType(new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0, 0])).mime, "image/png");
  assert.equal(previewType(new Uint8Array([0xff, 0xd8, 0xff, 0xe0])).mime, "image/jpeg");
  const webp = new Uint8Array([0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50]);
  assert.equal(previewType(webp).mime, "image/webp");
  assert.equal(previewType(new TextEncoder().encode("<svg onload=x>")).kind, "text", "never drawn as a picture");
});

test("the reader page forbids scripts and every load from outside", () => {
  const doc = readerDocument("<p>Hi</p>");
  assert.ok(doc.indexOf("Content-Security-Policy") < doc.indexOf("<style>"), "the policy comes before anything it governs");
  assert.match(READER_CSP, /default-src 'none'/);
  assert.match(READER_CSP, /img-src data:;/);
  assert.ok(!/script-src/.test(READER_CSP), "no script source at all");
  assert.ok(doc.includes('<base target="_blank">'));
  assert.ok(doc.endsWith("<body><p>Hi</p></body></html>"));
});

test("file sizes read like a mail program's", () => {
  assert.equal(fileSize(0), "");
  assert.equal(fileSize(800), "800 B");
  assert.equal(fileSize(150_000), "146 KB");
  assert.equal(fileSize(3_500_000), "3.3 MB");
});

test("a link that is a file is told from a page", () => {
  // marko's own: "Download receipt | Download invoice".
  assert.ok(isDownloadLink("Download receipt", "https://pay.stripe.com/receipts/payment/CAcaFwoV"));
  assert.ok(isDownloadLink("Download invoice", "https://invoice.stripe.com/i/acct_1/live_YWNj?s=em"));
  assert.ok(isDownloadLink("Stiahnuť faktúru", "https://shop.example.sk/f/1"));
  assert.ok(isDownloadLink("Rechnung herunterladen", "https://shop.example.de/r?id=2"));
  assert.ok(isDownloadLink("Receipt", "https://shop.example/r/1"));
  assert.ok(isDownloadLink("here", "https://shop.example/files/E-Tickets%20Oasis.pdf"), "the address ends in a file");
  assert.ok(isDownloadLink("Get it", "https://pay.example/receipts/payment/abc/pdf"));
  // The links in an ordinary mail stay links.
  assert.ok(!isDownloadLink("Concerts", "https://www.viagogo.com/concerts"));
  assert.ok(!isDownloadLink("help centre", "https://support.viagogo.com/hc/en-us"));
  assert.ok(!isDownloadLink("#1160703", "https://viagogo.zendesk.com/hc/requests/1160703"));
  assert.ok(!isDownloadLink("Download", "mailto:help@viagogo.com"), "only web addresses");
  assert.ok(!isDownloadLink("Questions about the receipt for your order? Our help centre has the answers", "https://x.example/help"));
  assert.ok(!isDownloadLink("Download", "not a link"));
});
