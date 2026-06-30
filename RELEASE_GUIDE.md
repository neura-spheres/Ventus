# Ventus Release Guide

Use this file when the user asks for a future Ventus release or prerelease build. The goal is to produce verified local release artifacts and good GitHub release text. Do not publish, push, upload, or create GitHub releases unless the user clearly asks for that in the same message.

## Scope

This repo is Ventus at `C:\Projects\NeuraSearch`.

For a release or prerelease request, the AI should usually do all of this:

1. Read the current codebase context and this file.
2. Check the latest GitHub releases to choose the correct tag and release name.
3. Confirm or update the version files.
4. Build the release binary.
5. Build the installer.
6. Verify the final files, version metadata, and SHA256 hash.
7. Write a GitHub-ready title and release description.
8. Tell the user exactly what needs to be uploaded manually.

The AI should not stop at only writing release notes if the user asked for a build. The expected output is a real local installer file.

## Do Not Publish Automatically

The user uploads releases manually.

Do not do these unless the user explicitly asks:

- Do not run `gh release create`.
- Do not upload assets to GitHub.
- Do not push tags.
- Do not push commits.
- Do not mark a release as latest.
- Do not publish a prerelease.

The normal handoff should stop after local verification and give the user the title, description, tag, installer path, hash, and manual next steps.

## Files To Check First

Always check the working tree before editing:

```powershell
git status --short --untracked-files=all
```

Read project rules before making edits:

```powershell
Get-Content RULES.md
```

Check these version surfaces:

```powershell
Select-String -Path Cargo.toml,config.yaml,Cargo.lock,installer\*.iss -Pattern 'version = "|version:|AppVersion'
```

The main version files are:

- `Cargo.toml`
- `Cargo.lock`, only the `name = "ventus"` package entry
- `config.yaml`
- `installer\ventus.iss`
- `installer\neura-search.iss`

The installer script reads `config.yaml` and passes `/DMyAppVersion=<version>` to Inno Setup, so `config.yaml` is the most important packaging version source. Still, keep the installer fallback versions aligned when doing a clean version bump.

## Stable Release Version Rules

Stable releases use plain semantic versions:

```text
1.0.36
```

Stable GitHub tags use a `v` prefix:

```text
v1.0.36
```

Stable installer file:

```text
dist\Ventus-Setup-1.0.36.exe
```

Stable release title format:

```text
Ventus v1.0.36 - Short Human Summary
```

Example:

```text
Ventus v1.0.36 - Smoother Loading, Smarter Right Click, and Better Site Compatibility
```

## Prerelease Version Rules

Prerelease builds use this format:

```text
<base-version>-pre<YYMMDD><NN>
```

Example for June 30, 2026:

```text
1.0.37-pre26063001
```

Prerelease GitHub tag:

```text
v1.0.37-pre26063001
```

Prerelease installer file:

```text
dist\Ventus-Setup-1.0.37-pre26063001.exe
```

Prerelease release title format:

```text
Ventus v1.0.37-pre26063001 - Short Human Summary
```

The build script handles prerelease naming with this code path:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1 -PreRelease
```

How it works:

- It reads the base version from `config.yaml`.
- It uses the current date as `YYMMDD`.
- It scans `dist` for existing installers matching `Ventus-Setup-<base>-pre<YYMMDD>*.exe`.
- It picks the next two digit iteration number.
- It sets `VENTUS_VERSION` for the build.
- It builds `target\release\ventus.exe`.
- It creates `dist\Ventus-Setup-<base>-pre<YYMMDD><NN>.exe`.

If the user asks for a specific prerelease iteration, use:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1 -PreRelease -Iteration 2
```

## Release And Commit Lookup

Before naming a release or writing notes, check the releases that already exist on GitHub:

```powershell
gh release list --repo neura-spheres/Ventus --limit 30
```

Use this to decide:

- What the latest stable release is.
- What prerelease number comes next.
- Whether the new release should be stable or prerelease.
- Whether release notes should combine multiple prerelease notes into one stable note.
- What updates were already shipped, so the new notes do not repeat old release text as if it is new.

To inspect earlier prerelease notes:

```powershell
gh release view v1.0.36-pre26062902 --repo neura-spheres/Ventus --json name,tagName,body,publishedAt
```

For stable releases, combine all prerelease notes from that same version line. For example, stable `v1.0.36` should combine `v1.0.36-pre...` notes and any new user-provided changes.

Also check the pushed git history before writing the final notes:

```powershell
git fetch --tags
git log --oneline --decorate --max-count=40
```

When there is a clear previous release tag, inspect the commits after that tag:

```powershell
git log --oneline v1.0.36..HEAD
```

For prereleases, compare against the latest prerelease or stable tag that already exists. For stable releases, compare against the previous stable tag and also read the prerelease notes from the same version line. Use the commit history to catch code changes that were pushed but not described by the user.

Do not blindly copy commit messages into the release notes. Use them as evidence, then write the notes in user-facing language.

## Build Commands

Run formatting and checks first when practical:

```powershell
cargo fmt
```

If `src\ui\chrome.html` changed, verify the embedded JavaScript:

```powershell
@'
const fs = require('fs');
const s = fs.readFileSync('src/ui/chrome.html','utf8');
const m = s.match(/<script>([\s\S]*)<\/script>/);
if (!m) throw new Error('script block not found');
new Function(m[1]);
console.log('chrome inline script syntax OK');
'@ | node -
```

Run Rust verification:

```powershell
git diff --check
cargo check
cargo test
```

Build stable installer:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1
```

Build prerelease installer:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1 -PreRelease
```

The build script also runs the release build. It should produce:

```text
target\release\ventus.exe
dist\Ventus-Setup-<version>.exe
```

## Artifact Verification

After building, verify the files:

```powershell
$version = "1.0.36"
$rows = @()
foreach ($f in @("target\release\ventus.exe", "dist\Ventus-Setup-$version.exe")) {
    $item = Get-Item $f -ErrorAction Stop
    $vi = $item.VersionInfo
    $rows += [pscustomobject]@{
        Path = $item.FullName
        SizeMB = [math]::Round($item.Length / 1MB, 2)
        FileVersion = $vi.FileVersion
        ProductVersion = $vi.ProductVersion
        LastWriteTime = $item.LastWriteTime
    }
}
$rows | Format-List
Get-FileHash "dist\Ventus-Setup-$version.exe" -Algorithm SHA256 | Format-List
git status --short --untracked-files=all
```

For prereleases, set `$version` to the full prerelease version, like:

```powershell
$version = "1.0.37-pre26063001"
```

Always include the exact installer path and SHA256 in the final answer.

## If The Build Fails

If `cargo build` cannot overwrite the exe, stop only this repo's Ventus process:

```powershell
$p = Get-Process ventus -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq 'C:\Projects\NeuraSearch\target\release\ventus.exe' }
if ($p) { $p | Stop-Process -Force }
```

If debug builds fail with weird `.llvm` unresolved external symbols, the incremental cache may be corrupted. Clear the relevant incremental folder and retry:

```powershell
Remove-Item -Recurse -Force target\debug\incremental
```

Only use broader cleanups if the smaller fix does not work.

## Release Note Style

Use release writing that sounds like a college student wrote it:

- Casual but still clear.
- Direct and human.
- Good GitHub markdown formatting.
- No emotes.
- No em dash.
- No corporate wording.
- No overexplaining internal code unless it helps users understand the change.
- Do not say things that were not actually changed or verified.

Avoid:

```text
This release leverages robust architectural improvements to deliver enhanced functionality.
```

Use:

```text
This release makes Ventus feel smoother when pages are loading, fixes a few site compatibility issues, and cleans up some browser behavior that was getting annoying.
```

## Release Note Format

Use this structure for GitHub:

```md
This release brings together the latest work on Ventus. It focuses on smoother loading, better site compatibility, and small browser fixes that make daily browsing feel less annoying.

## What's changed

### Smoother loading
- Fixed a case where restored pages could stay stuck on a black screen.
- Improved loading behavior on heavy media pages.
- Made app-style pages recover more safely without forcing the page back to the start.

### Better site compatibility
- Fixed sites that rejected Ventus because the browser identity looked inconsistent.
- Updated browser headers so more sites treat Ventus like a normal Chrome-compatible browser.
- Fixed right-click menu placement when page zoom is not 100%.

### Cleaner browser behavior
- Websites now get the first chance to handle right clicks.
- Bookmark folder popups close more reliably.
- Favicons load from local cache when available.
```

For prereleases, start with:

```md
This prerelease focuses on <main thing>. It should make <user-facing result> feel better while we keep testing the next stable build.

## What's changed

### <Area 1>
- <Change>
- <Why it matters>

### <Area 2>
- <Change>
- <Why it matters>
```

For stable releases that combine prereleases, start with:

```md
This release brings together all the v1.0.36 prerelease work into one stable update. It focuses on <main themes>.

## What's changed

### <Theme>
- <Combined change from prereleases>
- <User-facing impact>
```

## Final Answer Format

The final answer should be short but complete. Use this shape:

```text
Built the <stable/prerelease> release.

Tag:
`v1.0.36`

Release build:
`C:\Projects\NeuraSearch\target\release\ventus.exe`

Installer:
`C:\Projects\NeuraSearch\dist\Ventus-Setup-1.0.36.exe`

SHA256:
`...`

Verified:
`cargo fmt`, chrome inline JS syntax check if needed, `git diff --check`, `cargo check`, `cargo test`, and `scripts\build-installer.ps1`.

Release title:
`Ventus v1.0.36 - Smoother Loading, Smarter Right Click, and Better Site Compatibility`

Release description:
<paste GitHub markdown release notes here>

What you need to do:
1. Create the GitHub release with tag `v1.0.36`.
2. Upload `C:\Projects\NeuraSearch\dist\Ventus-Setup-1.0.36.exe`.
3. Paste the title and description above.
4. Mark it as latest for stable releases, or prerelease for prerelease builds.
```

If the worktree has unrelated changes, say that clearly. Do not revert them.

## Manual Upload Checklist

Tell the user to do this after the local build is ready:

1. Open GitHub releases for `neura-spheres/Ventus`.
2. Draft a new release.
3. Use the tag from the AI response.
4. Use the title from the AI response.
5. Paste the release description.
6. Upload the verified installer from `dist`.
7. For stable releases, publish as latest and do not mark prerelease.
8. For prerelease builds, mark it as prerelease.

## Stable Release Example

Title:

```text
Ventus v1.0.36 - Smoother Loading, Smarter Right Click, and Better Site Compatibility
```

Description:

```md
This release brings together all the v1.0.36 prerelease work into one stable update. It focuses on making Ventus feel smoother, fixing some annoying site compatibility problems, and cleaning up browser behavior that was getting in the way.

## What's changed

### Smoother loading and recovery
- Improved loading behavior on heavy media pages like YouTube.
- Fixed a case where restored pages could stay stuck on a black screen with the spinner still running.
- Made app-style pages recover more safely without forcing users back to the start of the site.

### Better site compatibility
- Fixed sites that rejected Ventus because the browser identity looked inconsistent.
- Updated browser headers so picky sites treat Ventus more like a normal Chrome-compatible browser.
- Fixed the right-click menu position when page zoom is not 100%.

### Cleaner browser behavior
- Websites now get the first chance to handle right clicks.
- Strict-blocked pages now show a real Ventus page instead of a black screen.
- Bookmark folder popups and bookmark bar dragging feel more reliable now.
```

## Prerelease Example

Title:

```text
Ventus v1.0.37-pre26063001 - Faster Startup and Cleaner Page Loading
```

Description:

```md
This prerelease focuses on making Ventus start cleaner and load pages with fewer weird edge cases. It is mostly a reliability pass before the next stable build.

## What's changed

### Faster startup
- Reduced extra work during startup so the first window can show sooner.
- Cleaned up a load path that could make the browser feel slower than it needed to.

### Better page loading
- Fixed a case where pages could show loading even after the useful work was already done.
- Made the loading state update more directly, so the UI feels less stuck.

### Small cleanup
- Kept the changes focused on loading and startup behavior.
- No settings reset is needed for this build.
```
