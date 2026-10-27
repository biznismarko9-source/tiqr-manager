// 2.118.0 - TIQR on a phone, end to end, against the Firebase emulators: an
// account made on the phone waits for approval, gets in once approved, live
// sync turns itself on, an event made on the phone reaches Firestore, an
// event another device wrote arrives on the phone, and both are still there
// after a reload. Not part of `npm test`: it needs the emulators, a browser
// and a phone build made for the emulators.
//
//   VITE_TIQR_EMULATORS=1 npx vite build --mode phone --outDir dist-phone-emulators
//   npx firebase-tools@15 emulators:exec --config firebase.emulator.json \
//     --only firestore,auth --project demo-tiqr "node scripts/phone-emulator-check.mjs"
//
// from the repository root, after `npm run build:phone` has built the Rust
// half (src/phone/pkg). Playwright and its Chromium have to be installed
// (`npm i -D playwright && npx playwright install chromium`).
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const require = createRequire(import.meta.url);
let playwright;
try {
  playwright = require("playwright");
} catch {
  console.error("Playwright is not installed: npm i -D playwright && npx playwright install chromium");
  process.exit(1);
}
const { chromium, devices } = playwright;

const SITE = process.env.TIQR_PHONE_SITE || "dist-phone-emulators";
const PORT = 4174;
const PROJECT = "demo-tiqr";
const FS = `http://127.0.0.1:8085/v1/projects/${PROJECT}/databases/(default)/documents`;
// The emulator's own key: what the Firebase console does when someone is
// approved, without the rules (which no app may get past for that).
const OWNER = { authorization: "Bearer owner", "content-type": "application/json" };

// The site as a host would serve it: files as they are, anything else the app.
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".png": "image/png", ".woff2": "font/woff2", ".webmanifest": "application/manifest+json" };
const server = createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname));
  let file = join(SITE, path);
  let body;
  try {
    body = await readFile(file);
  } catch {
    file = join(SITE, "index.html");
    body = await readFile(file);
  }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(body);
});
await new Promise((r) => server.listen(PORT, "127.0.0.1", r));
const APP = `http://127.0.0.1:${PORT}/`;

const fsGet = async (path) => (await fetch(`${FS}/${path}`, { headers: OWNER })).json();
let ok = true;
const check = (cond, what) => {
  console.log(`${cond ? "ok  " : "FAIL"} ${what}`);
  if (!cond) ok = false;
};

// A clean start: no accounts, no documents from an earlier run.
await fetch(`http://127.0.0.1:9099/emulator/v1/projects/${PROJECT}/accounts`, { method: "DELETE" });
await fetch(`http://127.0.0.1:8085/emulator/v1/projects/${PROJECT}/databases/(default)/documents`, { method: "DELETE" });

const browser = await chromium.launch();
const page = await (await browser.newContext({ ...devices["iPhone 13"] })).newPage();
const problems = [];
page.on("pageerror", (e) => problems.push(e.message));
try {
  await page.goto(APP);
  await page.getByRole("button", { name: "Sign up" }).first().click();
  await page.getByPlaceholder("Your name").fill("Phone Check");
  await page.getByPlaceholder("you@example.com").fill("phone.check@example.com");
  const passwords = page.getByPlaceholder("********");
  await passwords.nth(0).fill("phone-check-123");
  await passwords.nth(1).fill("phone-check-123");
  await page.locator("form").getByRole("button", { name: "Create account" }).click();
  await page.getByText("Account pending approval").waitFor({ timeout: 20_000 });
  check(true, "an account made on the phone waits for approval");

  let users = {};
  for (let i = 0; i < 40 && !users.documents?.length; i++) {
    users = await fsGet("users");
    if (!users.documents?.length) await page.waitForTimeout(250);
  }
  const uid = users.documents?.[0]?.name.split("/").pop();
  check(!!uid && users.documents[0].fields.approved.booleanValue === false, "it is stored as pending");
  await fetch(`${FS}/users/${uid}?updateMask.fieldPaths=approved`, {
    method: "PATCH",
    headers: OWNER,
    body: JSON.stringify({ fields: { approved: { booleanValue: true } } }),
  });

  await page.reload();
  await page.locator("[data-phone-nav]").waitFor({ timeout: 30_000 });
  check(true, "approved, it opens TIQR");

  let devicesSeen = {};
  for (let i = 0; i < 40 && !devicesSeen.documents?.length; i++) {
    await page.waitForTimeout(500);
    devicesSeen = await fsGet(`users/${uid}/devices`);
  }
  check(devicesSeen.documents?.length === 1, "live sync turned itself on for the phone");

  await page.goto(`${APP}#/events`);
  await page.getByRole("button", { name: "New Event" }).first().click();
  await page.locator(".fixed.inset-0 input:not([type=file]):not([type=date])").first().fill("Made on the phone");
  await page.getByRole("button", { name: "Create event" }).click();
  await page.getByText("Made on the phone").first().waitFor({ timeout: 15_000 });
  let rows = {};
  for (let i = 0; i < 40; i++) {
    rows = await fsGet(`users/${uid}/rows`);
    if (rows.documents?.some((d) => d.fields.body?.stringValue?.includes("Made on the phone"))) break;
    await page.waitForTimeout(500);
  }
  check(!!rows.documents?.some((d) => d.fields.body?.stringValue?.includes("Made on the phone")), "an event made on the phone reached Firestore");

  // What another device's live sync writes (lib/liveSyncCore.ts, toStored).
  const now = Date.now();
  const u = "another-device-event-1";
  const body = JSON.stringify({ uid: u, name: "Made on the computer", venue: "O2 Arena", city: "Prague", country: "CZ", event_date: "2026-11-20", status: "upcoming", created_at: new Date(now).toISOString(), updated_at: new Date(now).toISOString(), is_demo: 0 });
  const written = await fetch(`${FS}/users/${uid}/rows/events~${u}`, {
    method: "PATCH",
    headers: OWNER,
    body: JSON.stringify({
      fields: {
        t: { stringValue: "events" },
        u: { stringValue: u },
        h: { integerValue: String(now) },
        del: { booleanValue: false },
        dev: { stringValue: "another-device" },
        at: { timestampValue: new Date().toISOString() },
        body: { stringValue: body },
      },
    }),
  });
  check(written.ok, "another device's event is in Firestore");
  const arrived = await page.getByText("Made on the computer").first().waitFor({ timeout: 30_000 }).then(() => true, () => false);
  check(arrived, "it arrives on the phone");

  await page.reload();
  const kept = await page.getByText("Made on the computer").first().waitFor({ timeout: 30_000 }).then(() => true, () => false);
  check(kept && (await page.getByText("Made on the phone").count()) > 0, "both are still there after a reload");
} catch (e) {
  ok = false;
  console.log("FAIL", e.message);
}
for (const p of problems) console.log("page error:", p);
await browser.close();
server.close();
console.log(ok && problems.length === 0 ? "phone check passed" : "phone check FAILED");
process.exit(ok && problems.length === 0 ? 0 : 1);
