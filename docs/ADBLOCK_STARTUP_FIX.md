# First-page adblock startup fix

The reported pattern was an ad on the first YouTube video, with later videos apparently filtered. That points to initialization timing; the earlier generic advertising-request test did not verify protection on the very first navigation.

## Findings

Normal-profile navigation could begin before asynchronous extension setup completed. Only the private-profile YouTube path previously deferred navigation, and even that used native extension installation/enabling completion as its readiness signal.

A live profile also retained the previous bundle's content-script registrations (`ublock-filters.main`, `easyprivacy.main`, and related entries). The current bundle expects `scriptlets-main` and `scriptlets-isolated`. Enabling an already-installed extension did not refresh those registrations.

## Change

- Normal and private profiles have independent startup gates.
- The first protected navigation waits for extension initialization. Native navigation and re-enabling filtering on an existing page also start the gate.
- At initialization, Ventus re-adds the installed unpacked extension from the current bundle path. [Microsoft documents this as reinstalling the same extension](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2profile7#addbrowserextension), which refreshes its loaded code and registrations. User browser cookies and profile data are not cleared.
- A hidden extension-origin WebView sends uBOL's existing `getOptionsPageData` request, which waits for upstream background initialization. It then verifies broad permissions, Optimal/Complete mode, enabled filtering rules, and the current main-world document-start scriptlets.
- Only a successful readiness result releases queued URLs. Generation checks reject stale callbacks. A 15-second timeout or failed readiness check releases the page with a visible warning rather than leaving it blank or claiming protection is ready.
- Hidden readiness views close before profile shutdown so they cannot retain profile locks.
- The upstream uBOL bundle is unchanged.

## Verification

The live normal-profile startup initially reproduced the stale registrations and reported readiness false. After adding the native refresh, the log showed the page held, extension installation completed, six rulesets enabled, Optimal filtering with broad permissions, and the expected current scriptlets registered. Readiness became true before the queued page navigation started.

Regression coverage includes waiting for worker initialization, rejecting missing/obsolete script registrations, Basic mode and missing rules, retrying an unavailable worker, stale native completions, late timeouts, and separate normal/private readiness.

`cargo fmt --check`, `cargo check --locked`, all 147 Rust tests, all 19 JavaScript tests, and `git diff --check` passed.

The final `cargo build --release --locked` and Inno Setup compilation passed. The optimized release's startup log confirmed readiness true, six enabled rulesets, current main-world scriptlets, and queued navigation beginning only after readiness. The local installer is `dist/patches/Ventus-Setup-1.0.39-startup-fix.exe`; its checksum is in `dist/patches/SHA256SUMS.txt`. The six cloud settings remain embedded in the release build.

Automatic approval review blocked remote-debugging and URL-driven test launches with the generic reason `blocked by policy`. Normal startup was verified through application logs instead. Actual first-video YouTube ad playback still needs a manual cold-start retest; ads vary by account and server response, so this is not a guarantee that every future YouTube ad format is blocked.

This patch is local. The already-published GitHub v1.0.39 artifacts have not been replaced. A separate `Ventus-Setup-1.0.39-startup-fix.exe` is intended for manual retesting before the next release.

## Follow-up, 2026-11-24

- Bundled uBOL is now the official `uBOLite_2026.1122.1948.chromium.zip` (SHA-256 `7cd72ae78fe6fa007193de633d2dec085a0da50f53283e821918a77978d105fb`).
- Site exceptions are written to uBOL's no-filtering list. The extension is disabled only by the global setting.
- A disabled extension is removed and reinstalled when turned back on, because its worker otherwise never answers.
- After a failed start, pages load immediately and retries run in the background. Finished pages are not stopped or reloaded.
