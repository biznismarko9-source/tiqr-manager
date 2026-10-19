// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { isReading, lastSyncText, laneText, laneViews, syncHeadline } from "./mailSyncStatus.ts";
import type { MailLaneProgress, MailSyncProgress } from "./types.ts";

const fmt = (n: number) => String(n);

function lane(waiting: number, discovery: MailLaneProgress["discovery"]): MailLaneProgress {
  return { waiting, discovery };
}

function progress(over: Partial<MailSyncProgress> = {}): MailSyncProgress {
  return {
    reading: null,
    waiting: 0,
    foldersDone: 0,
    foldersTotal: 0,
    storedTotal: 0,
    phase: "idle",
    newMail: lane(0, "watching"),
    starred: lane(0, "complete"),
    labels: lane(0, "complete"),
    backlog: lane(0, "complete"),
    due: 0,
    retryLater: 0,
    lastSuccessAt: null,
    ...over,
  };
}

test("a lane never invents a total: one still being found says so", () => {
  assert.deepEqual(laneText(lane(1240, "discovering"), fmt, "x"), { text: "1240+ to download", tone: "active" });
  assert.deepEqual(laneText(lane(0, "discovering"), fmt, "x"), { text: "finding…", tone: "active" });
  assert.deepEqual(laneText(lane(12, "complete"), fmt, "x"), { text: "12 to download", tone: "active" });
  assert.deepEqual(laneText(lane(0, "complete"), fmt, "x"), { text: "all here", tone: "done" });
});

test("a lane not looked at yet says what it is waiting for", () => {
  assert.equal(laneText(lane(0, "pending"), fmt, "Starred").text, "after Starred");
  assert.equal(laneText(lane(3, "pending"), fmt, "labels").text, "3 to download, finding more after labels");
});

test("the lanes come in the order they are worked, and failures only when there are some", () => {
  const p = progress({
    newMail: lane(2, "watching"),
    starred: lane(40, "discovering"),
    labels: lane(0, "pending"),
    backlog: lane(0, "pending"),
  });
  const views = laneViews(p, fmt);
  assert.deepEqual(
    views.map((v) => v.key),
    ["new", "starred", "labels", "backlog"],
  );
  assert.equal(views[0].text, "2 to download");
  assert.equal(views[1].text, "40+ to download");
  assert.equal(views[2].text, "after Starred");
  assert.equal(views[3].text, "after labels");
  const withRetry = laneViews(progress({ retryLater: 3 }), fmt);
  assert.equal(withRetry[withRetry.length - 1].text, "3");
  assert.equal(laneViews(progress({ newMail: lane(0, "pending") }), fmt)[0].text, "from the first sync");
});

test("the headline says the priority lanes go first and history follows", () => {
  const base = { paused: false, failed: false, rateLimited: false, intervalSeconds: 300 };
  assert.match(syncHeadline({ ...base, progress: progress({ phase: "starred", reading: "Starred" }) }), /Starred first.*history follows/);
  assert.equal(
    syncHeadline({ ...base, progress: progress({ phase: "labels", reading: "NFL MEXICO" }) }),
    "Your labels next (NFL MEXICO) - history follows",
  );
  assert.equal(syncHeadline({ ...base, progress: progress({ phase: "backlog", reading: "Inbox" }) }), "History in the background · Inbox");
  assert.equal(syncHeadline({ ...base, progress: progress({ phase: "new" }) }), "Downloading new mail first");
  assert.equal(syncHeadline({ ...base, progress: progress() }), "Up to date · checks every 5 min");
  assert.equal(syncHeadline({ ...base, intervalSeconds: 0, progress: progress() }), "Up to date");
});

test("paused, failed and slowed down win over the lane", () => {
  const p = progress({ phase: "starred" });
  assert.equal(syncHeadline({ progress: p, paused: true, failed: true, rateLimited: true, intervalSeconds: 60 }), "Paused");
  assert.match(syncHeadline({ progress: p, paused: false, failed: true, rateLimited: true, intervalSeconds: 60 }), /see below/);
  assert.match(syncHeadline({ progress: p, paused: false, failed: false, rateLimited: true, intervalSeconds: 60 }), /slow down/);
  assert.equal(isReading(p, false, false), true);
  assert.equal(isReading(p, true, false), false);
  assert.equal(isReading(progress(), false, false), false);
});

test("last sync is a time today and a date before that", () => {
  const now = new Date(2026, 9, 19, 15, 0);
  assert.equal(lastSyncText(new Date(2026, 9, 19, 9, 5).toISOString(), now), "Last sync 09:05");
  assert.equal(lastSyncText(new Date(2026, 9, 17, 9, 5).toISOString(), now), "Last sync 17 Oct");
  assert.equal(lastSyncText(null, now), "Not synced yet");
  assert.equal(lastSyncText("nonsense", now), "Not synced yet");
});
