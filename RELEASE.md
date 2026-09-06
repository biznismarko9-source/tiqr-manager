# TIQR Manager — Release & Distribution

Everything about shipping a version. Nothing in here is a secret; the actual
secret values live only in GitHub repository settings.

---

## How to cut a release

```
1. Bump the version   (9 occurrences across 7 files - see below)
2. Run 1-CLICK-UPDATE.bat
3. Watch the Actions run
4. Verify the release page
```

`1-CLICK-UPDATE.bat` runs `release.ps1`, which mirrors this folder into a
fresh clone of the GitHub repo, cross-checks the version, commits, deletes and
re-pushes the tag. The tag push is what starts the build.

### The version lives in 7 files (9 occurrences)

`release.ps1` hard-stops if the first three disagree, but it cannot see the
rest — so bump all of them:

| File | What |
|---|---|
| `package.json` | `"version"` |
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` |
| `release.ps1` | `$Version` **and** `$CommitMsg` |
| `1-CLICK-UPDATE.bat` | title **and** echo line |
| `src-tauri/Cargo.lock` | the `tiqr-manager` package entry (regenerate with `cargo check`) |
| `package-lock.json` | the two root `""` entries (regenerate with `npm install --package-lock-only`) |

**Versions only ever go forward.** The Tauri updater compares version numbers
directly and will not offer or accept a repeat or a downgrade — even a release
that is purely a revert has to get a new, higher number.

> `$CommitMsg` in `release.ps1` **must not contain a double quote.** Windows
> PowerShell 5.1 cannot pass an embedded `"` to `git.exe` intact, and the
> commit fails with an unhelpful error. There is a guard right before the
> commit that catches this.

---

## What the pipeline does

`.github/workflows/release.yml` (renamed from `build-windows.yml` in 2.11.0,
which is why `release.ps1`'s "did the workflow survive the mirror" guard also
names `release.yml` — those two must always agree).

**On a `v*` tag push:**

```
prepare-release      delete any stale GitHub release for this tag
      ↓
release (matrix, one at a time)
      ├── windows-latest → NSIS .exe  + updater artifact + .sig
      └── macos-latest   → universal .dmg + updater artifact + .sig
      ↓
      publishes both to ONE GitHub Release, and writes latest.json
      ↓
verify-release       asserts the release really has an .exe, a .dmg, a
                     latest.json, both platform families in it, and a
                     non-empty signature and url for every entry
```

The matrix runs **one platform at a time** on purpose: both legs publish to
the same release and both rewrite `latest.json`, so serialising them removes
the race and makes the manifest deterministic.

**On the Actions "Run workflow" button:** an unsigned test build of both
platforms, uploaded as workflow artifacts. It never creates a release and
never touches `latest.json`, so existing installs cannot be offered a test
build.

---

## Repository secrets

`Settings → Secrets and variables → Actions`.

### Required for a real release

| Secret | Purpose |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Signs updater artifacts. **Without it, existing installs will refuse every update.** |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The key's password, if it has one. |

The matching **public** key is already in `src-tauri/tauri.conf.json` under
`plugins.updater.pubkey` — that one is meant to ship inside the app; it is
what verifies a downloaded update. The private key must never be committed,
never appear in an installer, and never be pasted into a report or an issue.

### Optional feature secrets

All of these are optional. Without any of them the build still succeeds and
the app reports that feature "isn't available in this build".

| Secret | Feature |
|---|---|
| `GOOGLE_SERVICE_ACCOUNT_JSON` | Google Sheets sync |
| `GOOGLE_OAUTH_CLIENT_ID` / `_SECRET` | "Sign in with Google" for Sheets access |
| `FIREBASE_GOOGLE_OAUTH_CLIENT_ID` / `_SECRET` | "Continue with Google" app sign-in |
| `ANTHROPIC_API_KEY` | AI category detection + AI Import Assistant |

### Optional macOS signing / notarization — **not configured today**

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` | base64 of the Developer ID `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | its password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | notarization (app-specific password) |

The workflow already passes all six through. Until they exist the `.dmg`
builds and installs fine but is **unsigned**, so macOS shows an
"unidentified developer" warning and the user has to right-click → Open the
first time. Removing that warning requires a paid Apple Developer account —
there is no way around it, and nothing here fakes one.

### Windows code signing — **not configured today**

The `.exe` is currently **unsigned**, so SmartScreen shows a "Windows
protected your PC" warning on first run (More info → Run anyway). Removing it
needs a real code-signing certificate (OV or EV) from a commercial CA — a paid
external prerequisite. No test or self-signed certificate is used, because a
self-signed one produces the same warning while *looking* signed.

**Note this is separate from updater signing.** Updater signing (above) is
configured and is what makes in-app updates secure. Code signing only affects
the first-run OS warning.

---

## Artifacts on a release

| Platform | Install | Updater |
|---|---|---|
| Windows | `*-setup.exe` (NSIS, per-user) | `*.nsis.zip` + `.sig` |
| macOS | `*.dmg` (universal: Apple Silicon + Intel) | `*.app.tar.gz` + `.sig` |
| Both | | `latest.json` |

`latest.json` is what the app reads. `verify-release` fails the run if it is
missing, incomplete, or has an entry without a signature.

---

## Checklists

### Developer

1. Bump the version in all 7 files
2. Run `1-CLICK-UPDATE.bat`
3. Actions run goes green — including **verify-release**
4. Release page has a `.exe`, a `.dmg` and `latest.json`
5. Install the `.exe` on a clean Windows machine
6. Open the `.dmg` on a Mac
7. From the **previous** version, check that the update is offered and installs

### User

**Windows** — download the `.exe` → run it → install → launch.
No Node, no Rust, no npm, no manual WebView2 install: the installer fetches
the WebView2 runtime itself if the machine does not already have it (every
Windows 11 machine does).

**macOS** — download the `.dmg` → drag TIQR Manager to Applications → launch.
First launch on an unsigned build: right-click → Open → Open.

**Updating** — open the app → the Dashboard shows "Update to vX.Y.Z", or
Settings → Software → Check for updates → Download & install. The app restarts
itself when it finishes.

---

## Your data is not touched by an update

An update replaces the application, never the database. The SQLite file stays
where it is, and the new version runs its forward-only migrations at startup
exactly as it would on any other launch. There is no reset path, and no
migration exists for the updater's sake.

If an update fails — download interrupted, no internet, invalid signature —
nothing is installed and the current version keeps running. A signature that
does not verify is refused before anything is applied.
