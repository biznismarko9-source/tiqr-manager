# Protected areas - notes for future sessions

A running checklist of things in this codebase that are easy to break, or
easy to forget, when touching the areas they cover - not a changelog (see
the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files for those, one per
release), just traps worth knowing about before working here again. New
entries go at the top, dated by the version that found them.

## 2.1.9 - Price Checker: hidden auto-check replaced by the Visible Scanner

The ENTIRE 2.1.1-2.1.8 hidden-WebView auto-check line (`price_checker_auto*`)
is deleted, not just disabled - marko's own explicit call after it kept
being unreliable no matter how the extraction logic was patched. In its
place: `commands/price_checker_scanner.rs` (session state + the 4 Tauri
commands) and `commands/price_checker_scan.js` (the injected 3-layer
extraction script) open a real, visible `WebviewWindow` the user drives
themselves. Read that module's own doc comment first for the session model
and status-derivation rules before changing anything here. A few things
that bit during this rewrite, worth knowing before touching this area
again:

- **`WebviewWindow::eval_with_callback`'s result is encoded TWICE, not
  once, when the injected script itself returns `JSON.stringify(x)`.**
  `eval_with_callback`'s own doc comment says the JS completion value "will
  be serialized into a JSON string" for the callback - true, but
  `price_checker_scan.js`'s completion value is ITSELF already a JSON
  string (`return JSON.stringify(payload)`), so the callback's raw `String`
  is a JSON string LITERAL wrapping another JSON string. A single
  `serde_json::from_str::<ScanJsPayload>(&raw)` fails on literally every
  real scan with "invalid type: string ..., expected struct ScanJsPayload"
  - it silently never worked until caught by REAL runtime verification (a
  genuine `WebviewWindow` under Xvfb; no pure-Rust unit test or jsdom-only
  JS test can reach this, since both sides deserialize correctly in
  isolation - only the real Tauri wire format has the extra layer). Fixed
  in `parse_scan_js_payload` (two `serde_json::from_str` calls, unwrap the
  outer JSON string first) - if the injected script is ever changed to
  return a plain object completion value instead of a stringified one,
  this needs to come out again, and re-verify with a real WebviewWindow run
  when you do, not just the unit tests.
- **A session's `cancel_flag` must be REPLACED (a new `Arc`), never reset
  in place, at the start of each scan attempt.** Resetting the same shared
  `AtomicBool` back to `false` when a new scan starts can silently undo an
  in-flight scan's own Stop: scan A is blocked waiting on
  `eval_with_callback`, the user clicks Stop (sets the flag true), then
  clicks Scan again before A's result arrives - if the new attempt just
  flips the SAME flag back to false, A's own cancel check (after its eval
  finally returns) wrongly reads "not cancelled" and merges in a result the
  user explicitly stopped. `scan_visible_prices` now does
  `session.cancel_flag = Arc::new(AtomicBool::new(false))` so each attempt
  owns an independent flag; `cancel_price_scan`/`finish_session` still just
  flip whatever the CURRENT one is.
- **The dedup fingerprint is a literal composite of marketplace + price +
  currency + section + row + quantity + listing id (`fingerprint_for`,
  same file)** - every field feeds the key, missing ones contribute an
  empty slot. This means a listing whose section/row genuinely isn't
  visible yet on one scan, then becomes visible on a later scan of the
  SAME physical listing, is counted as a SECOND listing rather than
  recognized as the same one maturing from "partial" to "success" - an
  accepted, documented trade-off of marko's own literal fingerprint spec,
  not something silently patched over with a heuristic (a heuristic here
  risks the opposite, worse mistake: wrongly merging two genuinely
  different listings that happen to share a price, e.g. a
  general-admission listing and a specific-seat listing). If this ever
  needs real fixing, it needs actual cross-scan listing identity, which
  most real pages don't expose - not a quick tweak.
- **Proving this feature actually works needs a REAL `WebviewWindow`, and
  that needs `Builder::any_thread()` plus `App::run_return` under
  `xvfb-run -a`, run as a temporary/throwaway test.** `cargo test`'s
  harness runs each `#[test]` on a pooled worker thread, not the true OS
  main thread that `tao`'s Linux event loop insists on by default -
  `Builder::any_thread()` is the documented escape hatch, and it's only
  safe/appropriate for a test harness like this, never for the real
  shipped app (which legitimately starts on the real main thread via
  `fn main`). `App::run_return(callback)` (not `.run()`, which never
  returns) pumps the real event loop on the calling thread and hands control
  back once something calls `AppHandle::exit()` - drive the actual
  `#[tauri::command]` functions directly from a spawned thread inside
  `.setup()` (they're plain callable Rust functions), poll `AppState`/
  listen for the real Tauri events, and DELETE the throwaway test module
  again once it's done its job - this is not something to leave enabled in
  the permanent suite (it opens a real window and binds a real local
  socket for a fixture HTTP server; keep it out of ordinary `cargo test`
  runs, including `--ignored`, by not committing it at all).

## 2.1.8 - Price Checker real-DOM reader rewrite

`price_checker_auto_extract.js`/`price_checker_auto_readiness.js` were
rewritten around per-marketplace readers (StubHub/Vivid Seats/Ticombo, each
with its own layered selectors, all falling through to `readGeneric()` as a
last resort) instead of one generic parser, plus a bounded multi-attempt
retry loop in `poll_then_extract`. A few things that bit during this work,
worth knowing before touching this area again:

- **Never concatenate sibling elements' text without a separator when
  scanning for context.** `.textContent` on a parent with adjacent
  `<span>` children (exactly how JSX compiles
  `<span>{a}</span><span>{b}</span>`, with zero whitespace between them)
  runs their text together with NOTHING between them - "Row 12" next to "2
  tickets" reads as "122 tickets" and silently reports the wrong quantity,
  not just messy text. `nearbyListingContext` now uses `textWithGaps` (an
  element-boundary-aware walker) instead of plain `textOf`/`.textContent`
  for exactly this reason - if you add another context-scanning regex, feed
  it `textWithGaps`, not `textOf`.
- **A price parser is not done until it's been checked against a European
  decimal-comma format.** `parseMoney`'s original number parsing did
  `parseFloat(numRaw.replace(/,/g, ""))`, which silently treats a comma as
  a thousands separator always - correct for "$1,234.56", silently 100x-1000x
  wrong for "234,56 €" (marko is in Slovakia; this is the format he
  actually sees). `src/lib/priceParse.ts`'s `normalizeAmountToken` already
  had the correct locale-aware logic (look at the LAST separator and how
  many digits follow it) - `price_checker_auto_extract.js` now has its own
  plain-JS port of the same algorithm (can't `import` a TS module into a
  webview eval string). If either file's money parsing is touched again,
  re-verify BOTH "234,56 €" and "$1,234.56" still come out right, not just
  one format.
- **"ok" now specifically means "a price correlated with real section/row/
  seat context", not just "a price was found".** A bare, uncorrelated price
  is `"partial"` (own status, own amber banner in PriceChecker.tsx, still
  prefills the editable fields) - this rule applies to EVERY path that can
  produce an `AutoCheckResult`, including the AI-assisted fallback
  (`try_ai_extraction_fallback`), which structurally can never populate
  `listings` (its prompt/schema has no section/row/seat slot at all) and so
  can never legitimately be `"ok"` either. If a future change adds a new
  way to produce a result, it needs to honestly decide ok vs. partial by
  this same rule, not default to "ok" because prices exist.
- **An extraction attempt's own eval timeout must NEVER be derived from a
  shrinking remaining-budget clock** - this is the THIRD time this exact
  lesson mattered (2.1.6, 2.1.7, and again while adding the 2.1.8 retry
  loop, where the between-attempts readiness/scroll eval - not the
  extraction eval itself - needed a budget cap for a DIFFERENT reason: its
  result is always discarded, so shrinking IT is safe, but it still needed
  the cap to stop it silently adding up to a whole extra `EVAL_TIMEOUT` of
  overshoot on top of the documented ~63s ceiling). The rule stands: only
  a boolean "is this the last attempt" may depend on remaining budget: no
  eval whose RESULT is actually used may ever get a shrunken timeout.
- **Diagnostics text/attribute scrubbing is a defense-in-depth heuristic,
  not a guarantee** - `scrubSensitiveText`/`stripSuspiciousAttributeValues`
  in the extract script catch labeled patterns (Bearer/JWT/token=/session=)
  plus generic long opaque runs (all-digit 16+, all-letter 24+, mixed
  base64-alphabet 24+), but deliberately EXCLUDE hyphens/dots from the
  generic mixed-run check specifically so ordinary hyphenated section/row
  slugs ("grandstand-outfield-413") survive - don't widen that character
  class back to include hyphens without re-testing against real listing
  markup, or legitimate diagnostic detail silently disappears again.

## 2.1.6 - a version bump is not just the 3 JSON/TOML files

The obvious version-number locations are `package.json`, `src-tauri/tauri.conf.json`
and `src-tauri/Cargo.toml` (all three - `release.ps1` itself cross-checks
that all three agree, see below). It is easy to stop there and still ship a
broken or misleading release. Also check, every time:

- **`release.ps1`'s `$Version` constant.** This drives the actual git tag,
  and `release.ps1` HARD STOPS if `$Version` (with its `v` stripped) doesn't
  match what it finds in the 3 files above after mirroring this folder into
  a fresh clone - so forgetting to bump it is caught, but with a confusing
  "this clone does not actually have vX.Y.Z everywhere / $SourceDir is
  stale" message that points at the wrong cause. Bump `$Version` itself,
  don't rely on that check to remind you.
- **`release.ps1`'s `$CommitMsg`.** This is a fully static string, not
  generated from anything - it describes whatever release it was LAST
  written for. If it isn't rewritten, the git tag for the new version ships
  with a commit message describing the PREVIOUS release's changes instead.
  Easy to miss because nothing fails or warns - the script runs fine either
  way, it just publishes a misleading commit message forever.
- **`1-CLICK-UPDATE.bat`'s title/echo text.** Purely cosmetic (the actual
  release mechanics come entirely from `release.ps1`), but it has its own
  hardcoded `vX.Y.Z` strings that do not follow `release.ps1`'s `$Version`
  automatically - found still saying "v2.1.3" while 3 real releases (2.1.3,
  2.1.4, 2.1.5) had already shipped without it ever being updated.
- **`Cargo.lock` / `package-lock.json`.** Not hand-edited - after bumping
  `Cargo.toml`/`package.json`, run `cargo check` (regenerates the
  `tiqr-manager` entry in `Cargo.lock`) and `npm install --package-lock-only`
  (regenerates the root `""` package entries in `package-lock.json`,
  currently 2 of them) so the lockfiles don't silently drift from the
  manifests. `package-lock.json` also contains unrelated third-party
  packages that happen to share a version number with the app (e.g.
  `@nodelib/fs.scandir` was genuinely at `2.1.5` too) - don't touch those.

None of this is enforced by a test; it was found by grepping the whole repo
for the outgoing version string right before packaging 2.1.6 and reading
`release.ps1` in full rather than assuming its only version reference was
the obvious `$Version` line.
