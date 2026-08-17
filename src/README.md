# TIQR Manager

A local-first desktop app for ticket resellers: track events, orders, individual
tickets, sales, revenue, cost, profit, margin and ROI. Everything is stored in a
private SQLite database on your own machine — no server, no cloud, no account.

Built with Tauri 2 (Rust) + React + TypeScript + Tailwind + SQLite.

## Getting the Windows installer (`TIQR-Manager-Setup.exe`)

This source was built and fully tested on Linux, where a real Windows `.exe`
cannot be produced (Windows builds require Microsoft's toolchain). There are two
free ways to get the actual installer — pick whichever is easier for you.

### Option A — GitHub Actions (recommended, no Windows machine needed)

1. Push this folder to a new GitHub repository (public or private — both are free
   for this; a private repo gets 2,000 free build-minutes/month, plenty for this).
2. On GitHub, open the repo → **Actions** tab → **Build Windows Installer** →
   **Run workflow** button.
3. Wait about 10-15 minutes for the run to go green.
4. Open the finished run and scroll down to **Artifacts** → download
   **TIQR-Manager-Setup**. Unzip it — inside is `TIQR-Manager-Setup.exe`.

The workflow file is already in this repo at
`.github/workflows/build-windows.yml`; nothing else needs to be configured. It
also re-runs automatically on every future push to `main`, so new versions are
just as easy.

### Option B — Build directly on a Windows PC

If you have (or can borrow) a Windows 10/11 machine:

1. Copy this whole folder onto that machine.
2. Open PowerShell in the folder and run:
   ```powershell
   Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
   .\scripts\windows-build.ps1
   ```
3. The script installs whatever is missing (Node.js, Rust, the Visual Studio
   C++ Build Tools — all free, via `winget`), builds the app, and copies the
   finished `TIQR-Manager-Setup.exe` to your Desktop.

First run takes 15-30 minutes (mostly the C++ Build Tools download). The
script is safe to re-run if it stops partway through — it skips anything
already installed.

Either path produces the exact same installer: a normal Windows setup wizard
that installs TIQR Manager for the current user (no admin rights required),
with demo data ready to explore on first launch.

## Local development

```bash
npm install
npm run tauri dev     # launch the app in dev mode with hot reload
npm run build          # type-check + build the frontend only
npm run tauri build     # full production build for the current OS
```

The SQLite database lives in the OS-standard app-data folder (e.g.
`%APPDATA%\com.tiqrmanager.app` on Windows), never inside the install
directory, so it survives reinstalls/updates.

## Project layout

- `src/` — React + TypeScript frontend
- `src-tauri/` — Rust backend (Tauri commands, SQLite access, migrations)
- `src-tauri/migrations/` — versioned SQL schema migrations, starting at
  `001_initial_schema.sql`
- `.github/workflows/build-windows.yml` — CI that produces
  `TIQR-Manager-Setup.exe`
- `scripts/windows-build.ps1` — local Windows build script (alternative to CI)
