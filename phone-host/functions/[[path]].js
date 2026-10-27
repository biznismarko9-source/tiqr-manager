// 2.118.0: TIQR for phones on Cloudflare Pages (.github/workflows/phone-site.yml).
// Everything there is the site's own files, except this: Firebase's Google
// sign-in pages (/__/auth/..., and the settings they read, /__/firebase/...),
// fetched from Firebase and handed over as the site's own - the same as
// TIQR on a server did (src-tauri/src/web_access.rs, `sign_in_routes`).
//
// Google's sign-in hands back to the address it was opened from. From
// firebaseapp.com, a browser that keeps each site's storage to itself -
// Safari, and an iPhone's home-screen app above all - comes back without the
// sign-in; from here it is all one site (src/lib/firebase.ts). Firebase's own
// advice for exactly this: "proxy auth requests to firebaseapp.com".
//
// Only these two folders reach this (`_routes.json`, written by
// vite.config.ts), only Firebase's host is asked, and only what a page needs
// goes either way - no cookies.
const FIREBASE = "https://tiqr-manager-b890a.firebaseapp.com";

const ASKED_WITH = ["accept", "accept-language", "content-type", "user-agent", "if-none-match", "if-modified-since"];
const ANSWERED_WITH = ["content-type", "cache-control", "etag", "last-modified", "expires", "location", "vary", "content-language"];

const UNREACHABLE = "Google sign-in is out of reach for a moment. Go back and try again.";

export async function onRequest({ request }) {
  const url = new URL(request.url);
  const path = url.pathname;
  const auth = path.startsWith("/__/auth/");
  const settings = path.startsWith("/__/firebase/");
  if ((!auth && !settings) || path.split("/").includes("..") || path.includes("//")) {
    return new Response("Not found", { status: 404 });
  }
  const method = request.method;
  if (method !== "GET" && method !== "HEAD" && !(auth && method === "POST")) {
    return new Response("Method not allowed", { status: 405 });
  }
  const headers = new Headers();
  for (const name of ASKED_WITH) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }
  let answer;
  try {
    answer = await fetch(FIREBASE + path + url.search, {
      method,
      headers,
      body: method === "POST" ? await request.arrayBuffer() : undefined,
      redirect: "manual",
    });
  } catch {
    return new Response(UNREACHABLE, { status: 502, headers: { "content-type": "text/plain; charset=utf-8" } });
  }
  const out = new Headers();
  for (const name of ANSWERED_WITH) {
    const value = answer.headers.get(name);
    if (value) out.set(name, value);
  }
  return new Response(answer.body, { status: answer.status, headers: out });
}
