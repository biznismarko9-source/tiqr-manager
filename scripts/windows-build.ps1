#Requires -Version 5.1
<#
.SYNOPSIS
    One-shot setup + build script for TIQR Manager on Windows.

.DESCRIPTION
    Run this on a Windows 10/11 machine to produce TIQR-Manager-Setup.exe
    without needing GitHub Actions. It will:
      1. Check for / install Node.js LTS, Rust, and the Visual Studio C++
         Build Tools (all via winget, all free).
      2. Install npm dependencies.
      3. Run the Tauri release build (compiles the Rust backend, bundles the
         React frontend, produces the NSIS installer).
      4. Copy the finished installer to your Desktop as TIQR-Manager-Setup.exe.

    These tools are only needed on THIS build machine. The people who later
    run TIQR-Manager-Setup.exe on their own PCs need none of this.

.NOTES
    - Run from a normal PowerShell window (does not need to be Administrator,
      but winget installs may prompt for elevation individually).
    - Safe to re-run: every step skips itself if already satisfied.
    - Takes roughly 15-30 minutes the first time (mostly the VS Build Tools
      and the first Rust compile). Subsequent runs are much faster.
#>

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
    Write-Host ""
    Write-Host "==> $msg" -ForegroundColor Cyan
}

function Write-Ok($msg) {
    Write-Host "    OK: $msg" -ForegroundColor Green
}

function Write-Warn2($msg) {
    Write-Host "    NOTE: $msg" -ForegroundColor Yellow
}

function Test-Command($name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

function Refresh-Path {
    $machine = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    $user = [System.Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = "$machine;$user"
}

Write-Host "TIQR Manager - Windows build setup" -ForegroundColor White
Write-Host "===================================" -ForegroundColor White

# ---------------------------------------------------------------------------
# 0. winget availability
# ---------------------------------------------------------------------------
Write-Step "Checking for winget (Windows Package Manager)"
$hasWinget = Test-Command "winget"
if ($hasWinget) {
    Write-Ok "winget is available"
} else {
    Write-Warn2 "winget was not found. Automatic installs below will be skipped."
    Write-Warn2 "Install 'App Installer' from the Microsoft Store, or install Node.js, Rust and the Visual Studio Build Tools manually, then re-run this script."
}

# ---------------------------------------------------------------------------
# 1. Node.js
# ---------------------------------------------------------------------------
Write-Step "Checking for Node.js"
if (Test-Command "node") {
    $nodeVer = (node --version)
    Write-Ok "Node.js already installed ($nodeVer)"
} elseif ($hasWinget) {
    Write-Host "    Installing Node.js LTS via winget..."
    winget install --id OpenJS.NodeJS.LTS -e --source winget --accept-package-agreements --accept-source-agreements
    Refresh-Path
    if (Test-Command "node") {
        Write-Ok "Node.js installed ($(node --version))"
    } else {
        Write-Warn2 "Node.js was installed but is not yet on PATH in this window."
        Write-Warn2 "Close this PowerShell window, open a new one, and re-run this script."
        exit 1
    }
} else {
    Write-Warn2 "Node.js is required. Download it from https://nodejs.org (LTS version), install it, then re-run this script."
    exit 1
}

# ---------------------------------------------------------------------------
# 2. Rust
# ---------------------------------------------------------------------------
Write-Step "Checking for Rust (cargo)"
if (Test-Command "cargo") {
    $cargoVer = (cargo --version)
    Write-Ok "Rust already installed ($cargoVer)"
} elseif ($hasWinget) {
    Write-Host "    Installing Rust via winget..."
    winget install --id Rustlang.Rustup -e --source winget --accept-package-agreements --accept-source-agreements
    Refresh-Path
    # rustup installs to %USERPROFILE%\.cargo\bin, which may not be on PATH yet this session.
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path $cargoBin) {
        $env:Path = "$cargoBin;$env:Path"
    }
    if (Test-Command "cargo") {
        Write-Ok "Rust installed ($(cargo --version))"
    } else {
        Write-Warn2 "Rust was installed but is not yet on PATH in this window."
        Write-Warn2 "Close this PowerShell window, open a new one, and re-run this script."
        exit 1
    }
} else {
    Write-Warn2 "Rust is required. Download it from https://rustup.rs, install it, then re-run this script."
    exit 1
}

# ---------------------------------------------------------------------------
# 3. Visual Studio C++ Build Tools (required by the Rust MSVC toolchain)
# ---------------------------------------------------------------------------
Write-Step "Checking for the Visual Studio C++ Build Tools"
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasBuildTools = $false
if (Test-Path $vsWhere) {
    $vsInstalls = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsInstalls) { $hasBuildTools = $true }
}

if ($hasBuildTools) {
    Write-Ok "C++ Build Tools already installed"
} elseif ($hasWinget) {
    Write-Host "    Installing Visual Studio 2022 Build Tools (C++ workload) via winget."
    Write-Host "    This is the slowest step (several GB download) - please be patient."
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --source winget --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    Write-Ok "Visual Studio Build Tools install finished (or was already satisfied)"
    Write-Warn2 "If this was just installed for the first time, you may need to close this window, open a new PowerShell, and re-run this script once so the compiler is picked up."
} else {
    Write-Warn2 "The Visual Studio C++ Build Tools are required to compile the Rust backend."
    Write-Warn2 "Download the free installer from https://visualstudio.microsoft.com/visual-cpp-build-tools/"
    Write-Warn2 "and select the 'Desktop development with C++' workload, then re-run this script."
    exit 1
}

# ---------------------------------------------------------------------------
# 4. Project dependencies + build
# ---------------------------------------------------------------------------
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
Set-Location $projectRoot
Write-Step "Working directory: $projectRoot"

Write-Step "Installing npm dependencies (npm ci)"
npm ci
Write-Ok "Dependencies installed"

Write-Step "Building TIQR Manager (this compiles Rust in release mode - a few minutes)"
npx tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Build failed. Scroll up for the actual compiler/bundler error." -ForegroundColor Red
    exit 1
}

# ---------------------------------------------------------------------------
# 5. Locate and copy the installer
# ---------------------------------------------------------------------------
Write-Step "Locating the generated installer"
$nsisDir = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis"
$installer = Get-ChildItem -Path $nsisDir -Filter "*-setup.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1

if (-not $installer) {
    Write-Host ""
    Write-Host "Build finished but no installer .exe was found under:" -ForegroundColor Red
    Write-Host "  $nsisDir" -ForegroundColor Red
    Write-Host "Check the build output above for bundler errors." -ForegroundColor Red
    exit 1
}

$destDir = [Environment]::GetFolderPath("Desktop")
$destPath = Join-Path $destDir "TIQR-Manager-Setup.exe"
Copy-Item $installer.FullName $destPath -Force

$sizeMB = [math]::Round((Get-Item $destPath).Length / 1MB, 1)

Write-Host ""
Write-Host "===================================" -ForegroundColor White
Write-Host " Done! ($sizeMB MB)" -ForegroundColor Green
Write-Host " Installer copied to: $destPath" -ForegroundColor Green
Write-Host "===================================" -ForegroundColor White
Write-Host ""
Write-Host "Double-click TIQR-Manager-Setup.exe on your Desktop to install and launch TIQR Manager."
