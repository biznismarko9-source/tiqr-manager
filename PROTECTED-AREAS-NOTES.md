# Protected areas - notes for future sessions

A running checklist of things in this codebase that are easy to break, or
easy to forget, when touching the areas they cover - not a changelog (see
the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files for those, one per
release), just traps worth knowing about before working here again. New
entries go at the top, dated by the version that found them.

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
