// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { costText } from "./promoCleanup.ts";

const scan = (calls: number, inputTokens: number, outputTokens: number) => ({
  scanned: 0, junk: 0, unsure: 0, keep: 0, calls, inputTokens, outputTokens, remaining: 0,
});

test("what a cleanup cost reads in words", () => {
  assert.equal(costText(scan(0, 0, 0)), "no AI needed - all kept by TIQR's own rules");
  // Ten requests of sixty promotions: about 40 000 tokens in, 2 000 out.
  assert.equal(costText(scan(10, 40_000, 2_000)), "10 requests, 42,000 tokens - less than a cent");
  assert.equal(costText(scan(1, 200_000, 20_000)), "1 request, 220,000 tokens - about $0.03");
});
