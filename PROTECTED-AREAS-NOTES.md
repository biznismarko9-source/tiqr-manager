# Protected areas - notes for future sessions

A running checklist of things in this codebase that are easy to break, or
easy to forget, when touching the areas they cover - not a changelog (see
the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files for those, one per
release), just traps worth knowing about before working here again. New
entries go at the top, dated by the version that found them.

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
