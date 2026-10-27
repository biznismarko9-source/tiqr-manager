// 2.118.0: TIQR on a phone opens without a connection. Its own files are kept
// on the phone the first time it opens - the list of them is written in by
// vite.config.ts (`npm run build:phone`) - and each new version brings a new
// list; the old one is let go once the new one is in.
//
// The page itself comes from the network when there is one, so a new version
// is picked up at the next opening; everything else is named by its contents
// and comes from what is kept. Firebase's sign-in pages (/__/) and anything
// from another site are never kept. The data is not here at all: it is in the
// database TIQR's worker keeps (src/phone/worker.ts).
const VERSION = "__TIQR_VERSION__";
const FILES = __TIQR_FILES__;
const KEPT = `tiqr-${VERSION}`;
/** How long the page is waited for before the kept one opens instead. */
const PATIENCE_MS = 4000;

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(KEPT)
      .then((cache) => cache.addAll(FILES))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((names) => Promise.all(names.filter((n) => n.startsWith("tiqr-") && n !== KEPT).map((n) => caches.delete(n))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin || url.pathname.startsWith("/__/")) return;
  if (request.mode === "navigate") {
    event.respondWith(page(request));
    return;
  }
  event.respondWith(caches.match(request).then((kept) => kept || fetch(request)));
});

/** The newest page when the network answers in time; the kept one when not. */
async function page(request) {
  const cache = await caches.open(KEPT);
  const fresh = fetch(request).then((response) => {
    if (response.ok) cache.put("/", response.clone());
    return response;
  });
  const late = new Promise((resolve) => setTimeout(resolve, PATIENCE_MS, null));
  try {
    const first = await Promise.race([fresh, late]);
    if (first) return first;
  } catch {
    // No network: the kept page below.
  }
  const kept = await cache.match("/");
  return kept || fresh;
}
