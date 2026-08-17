# release.ps1 - reliably publish this folder as a new TIQR Manager release.
#
# WHY THIS EXISTS: manually copying changed files into an old repo folder is
# easy to get wrong (miss a file, leave a stale one) - and when that happens,
# "git add / commit / push / tag" all still SUCCEED, they just publish the
# WRONG (old/mixed) code, which is exactly what caused GitHub to keep serving
# the old version last time. This script removes the manual-copy step:
# it clones your repo fresh, WIPES it (except .git), copies this entire
# folder on top so the repo becomes an exact match of what you see here,
# and only then commits/tags/pushes - with a hard stop and a clear message
# the moment anything fails, instead of silently continuing.
#
# HOW TO RUN: right-click this file -> "Run with PowerShell". Or, in a
# PowerShell window opened inside this same folder: .\release.ps1
#
# You need `git` installed and already logged in to GitHub (the same way
# it worked for you when v1.3.2 was released). Nothing else.

$ErrorActionPreference = "Stop"

$Version = "v1.4.0"
$RepoUrl = "https://github.com/biznismarko9-source/tiqr-manager.git"
$SourceDir = $PSScriptRoot
$RepoDir = Join-Path $env:TEMP "tiqr-manager-release-clone"

function Step($msg) { Write-Host ""; Write-Host "==> $msg" -ForegroundColor Cyan }

try {
    Step "Checking git is available"
    git --version
    if ($LASTEXITCODE -ne 0) { throw "git is not available in this terminal." }

    Step "Cloning a fresh copy of the repo into a temp folder"
    if (Test-Path $RepoDir) { Remove-Item -Recurse -Force $RepoDir }
    git clone $RepoUrl $RepoDir
    if ($LASTEXITCODE -ne 0) { throw "git clone failed - check the URL and that you have access to the repo." }

    Step "Wiping the clone's contents (keeping .git) so nothing stale survives"
    Get-ChildItem -Path $RepoDir -Force -Exclude ".git" | Remove-Item -Recurse -Force

    Step "Copying this folder's files into the clone"
    Copy-Item -Path (Join-Path $SourceDir "*") -Destination $RepoDir -Recurse -Force
    $ScriptCopy = Join-Path $RepoDir "release.ps1"
    if (Test-Path $ScriptCopy) { Remove-Item $ScriptCopy -Force }

    Set-Location $RepoDir

    Step "Checking whether the code itself changed vs what's already on GitHub"
    git add -A
    $Changes = git status --porcelain --cached
    if ([string]::IsNullOrWhiteSpace($Changes)) {
        # The main branch already has this exact code (e.g. a previous run got
        # the code there but the tagged release build never went green). That
        # is NOT a reason to stop - the thing still missing is a working
        # tagged release, so fall through and (re)tag + (re)push below.
        Write-Host "No file differences - this code is already on the main branch." -ForegroundColor Yellow
        Write-Host "Skipping commit/push to main. Moving on to (re)creating the $Version tag," -ForegroundColor Yellow
        Write-Host "in case the release build for it never actually succeeded." -ForegroundColor Yellow
    }
    else {
        Write-Host "Files that will be published:"
        git status --short --cached

        Step "Committing"
        git commit -m "$Version - refunds, multi-currency safety, delete safety, CSV seats"
        if ($LASTEXITCODE -ne 0) { throw "git commit failed - see the message above." }

        Step "Pushing to main"
        git push
        if ($LASTEXITCODE -ne 0) { throw "git push failed - see the message above (often a login/auth problem)." }
    }

    Step "Deleting any existing $Version tag on GitHub (so the push below is never a no-op)"
    # A plain "push --force" of a tag that already points at this exact commit
    # prints "Everything up-to-date" and pushes NOTHING - which means GitHub
    # never sees a new event and never re-runs the build. Deleting the remote
    # tag first guarantees the push after this is always a real, new event.
    git push origin --delete $Version 2>$null
    # (ignore the exit code here - it's fine if the tag didn't exist remotely yet)

    Step "Creating $Version locally (force) and pushing it as a fresh tag"
    git tag -f $Version
    if ($LASTEXITCODE -ne 0) { throw "git tag failed - see the message above." }
    git push origin $Version
    if ($LASTEXITCODE -ne 0) { throw "Pushing the tag failed - see the message above (often a login/auth problem)." }

    Write-Host ""
    Write-Host "Done. That tag push just fired a brand new GitHub Actions build of the signed installer." -ForegroundColor Green
    Write-Host "Watch it here: https://github.com/biznismarko9-source/tiqr-manager/actions"
    Write-Host "It takes a few minutes. Click into the newest run - if it ends with a red X instead of a" -ForegroundColor Green
    Write-Host "green check, open it, find the failed step, and send me exactly what it says." -ForegroundColor Green
}
catch {
    Write-Host ""
    Write-Host "STOPPED: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Nothing further ran - fix the issue above and run this script again." -ForegroundColor Red
    exit 1
}
