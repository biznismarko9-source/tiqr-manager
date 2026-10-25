import { test } from "node:test";
import assert from "node:assert/strict";
import { clearDiagnostics, diagnostics, diagnosticsText, redact, reportError } from "./diagnostics.ts";

test("nothing that could open an account or identify a person survives redaction", () => {
  const samples: [string, string][] = [
    ["mail to marko.novak+tickets@gmail.com failed", "marko.novak"],
    ["GET https://x.example/cb?code=4/0AeanS0aB1cD2&state=abc", "4/0AeanS0aB1cD2"],
    ["refresh_token=1//09abcdefGHIJKLmnopQRST and more", "1//09abcdefGHIJKLmnopQRST"],
    ['{"access_token":"ya29.a0AfH6SMBxyz123","expires_in":3599}', "ya29.a0AfH6SMBxyz123"],
    ["Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjM0In0.c2lnbmF0dXJlLXZhbHVl", "eyJhbGciOiJSUzI1NiJ9"],
    ["key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123 rejected", "sk-ant-api03"],
    ["password: hunter2!", "hunter2"],
    ["cookie=SID=31d4d96e407aad42; Path=/", "31d4d96e407aad42"],
    ["uid 3fa2c1d0e9b84f6a8c7d6e5f4a3b2c1d not found", "3fa2c1d0e9b84f6a8c7d6e5f4a3b2c1d"],
    ["picture data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB was too big", "iVBORw0KGgo"],
  ];
  for (const [text, secret] of samples) {
    const out = redact(text);
    assert.ok(!out.includes(secret), `${text} -> ${out}`);
  }
  // What is not a secret is left readable.
  assert.equal(redact("Event #12 not found"), "Event #12 not found");
  assert.ok(redact("x".repeat(5000)).length <= 500, "and nothing is kept at any length");
});

test("a failure is kept with when, where, how bad and an id - and a repeat is counted, not repeated", () => {
  clearDiagnostics();
  const t0 = new Date("2026-10-24T12:00:00Z");
  const a = reportError("artwork map", new Error("database is locked"), { now: t0 });
  assert.match(a.id, /^E-[2-9A-Z]{4}$/);
  assert.equal(a.severity, "error");
  const again = reportError("artwork map", new Error("database is locked"), { now: new Date(t0.getTime() + 5_000) });
  assert.equal(again, a);
  assert.equal(a.count, 2);
  // A minute later it is news again.
  reportError("artwork map", new Error("database is locked"), { now: new Date(t0.getTime() + 61_000) });
  reportError("page Events", "boom", { severity: "fatal", now: t0 });
  const kept = diagnostics();
  assert.equal(kept.length, 3);
  assert.equal(kept[2].severity, "fatal");
  const text = diagnosticsText();
  assert.match(text, /ERROR \[E-[2-9A-Z]{4}\] artwork map: database is locked \(x2\)/);
});

test("a failure carrying a secret is kept without it", () => {
  clearDiagnostics();
  reportError("google sign-in", new Error("token=abc123def456 for someone@example.com"));
  const [e] = diagnostics();
  assert.ok(!e.message.includes("abc123def456"));
  assert.ok(!e.message.includes("someone@example.com"));
});

test("the ring keeps the newest 200", () => {
  clearDiagnostics();
  for (let i = 0; i < 260; i++) reportError(`step ${i}`, "failed");
  const kept = diagnostics();
  assert.equal(kept.length, 200);
  assert.equal(kept[0].where, "step 60");
});
