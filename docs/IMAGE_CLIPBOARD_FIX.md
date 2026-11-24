# Copy image clipboard fix

The report concerned right-click → Copy image on Twitter/X.

## Findings and change

The custom menu dispatched `CopyImage` with the captured image URL correctly. The Windows writer, however, called `OpenClipboard(NULL)` before `EmptyClipboard` and `SetClipboardData`. [Microsoft documents this as a failing ownership sequence](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard). Clipboard writes now receive Ventus's actual window handle, including writes performed by asynchronous worker threads. The native text writer uses the same corrected ownership and memory-transfer helpers. Allocated memory is freed if ownership transfer fails.

The image fetch path also used `credentials: include` for all origins. A public Twitter CDN image returned `Access-Control-Allow-Origin: *` without credential permission, which cannot satisfy a credentialed cross-origin browser request. Public cross-origin images now use anonymous fetch first; same-origin images retain cookies, and protected cross-origin images can retry with credentials. The existing native HTTP fallback remains available.

## Verification

- `cargo fmt --check` and `cargo check --locked` passed.
- 148 regular Rust tests passed; the desktop clipboard test is ignored in the default suite to avoid modifying a developer's clipboard unexpectedly.
- The real Windows clipboard test passed separately. It creates an owner window, writes from another thread, verifies the DIB dimensions and pixel bytes, decodes the clipboard PNG, and checks text copying. The same test also successfully wrote a JPEG downloaded from the public Twitter image CDN.
- The previous clipboard was backed up in memory and restored after each native test. Its contents were not printed or saved to disk.
- All 23 JavaScript tests passed, including actual menu dispatch, anonymous Twitter CDN fetching, same-origin credentials, and protected cross-origin retry.
- The optimized release build and Inno Setup compilation passed. The combined local installer is `dist/patches/Ventus-Setup-1.0.39-startup-clipboard-fix.exe`, with its SHA-256 in the adjacent `.exe.sha256` file. The patched release app was relaunched normally.

This is a local patch alongside the adblock startup fix. No GitHub release assets or repository changes were pushed. A manual check of the originally affected Twitter image remains useful because its exact URL and reported toast were not provided.
