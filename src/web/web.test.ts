import { test } from "node:test";
import assert from "node:assert/strict";
import { parseFrame } from "./frames.ts";
import { failure, UNREACHABLE } from "./answers.ts";

test("a server-sent event is read as its name and data, a keep-alive as nothing", () => {
  assert.deepEqual(parseFrame('event: mail-sync\ndata: {"stored":2}'), { event: "mail-sync", data: '{"stored":2}' });
  assert.deepEqual(parseFrame("data: first\ndata: second"), { event: "message", data: "first\nsecond" });
  assert.deepEqual(parseFrame("event:no-space\ndata:x"), { event: "no-space", data: "x" });
  assert.equal(parseFrame(": keep-alive"), null);
  assert.equal(parseFrame(""), null);
  assert.equal(parseFrame("event: mail-sync"), null, "no data, nothing to deliver");
});

test("a refusal reads as the server worded it, a dead tunnel as unreachable", async () => {
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
  assert.equal(await failure(json(422, { error: "That sale is already paid." })), "That sale is already paid.");
  assert.equal(
    await failure(json(403, { error: "TIQR on the server is open for another account." })),
    "TIQR on the server is open for another account.",
  );
  // Cloudflare's own page when TIQR is not running behind the tunnel.
  assert.equal(await failure(new Response("<html>Bad gateway</html>", { status: 502 })), UNREACHABLE);
  assert.equal(await failure(new Response("<html>Tunnel</html>", { status: 530 })), UNREACHABLE);
  assert.equal(await failure(new Response("teapot", { status: 418 })), "TIQR on the server answered 418.");
});
