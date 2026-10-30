# Integrated Developer Tools

F12 and Ctrl+Shift+I toggle the active tab's bottom panel. Right-click → Inspect reveals the captured element in Elements. Ctrl+Shift+C and the panel's Select element button start the native element picker. Drag the divider, or focus it and use the arrow keys, to resize the panel.

## Architecture

`src/runtime/devtools.rs` creates a normal Wry child WebView for Microsoft's DevTools frontend. It does not open or reparent a native DevTools window. `AppLayout::calculate()` allocates the panel and resizes the page above it. Each inspected tab has a separate frontend and protocol connection. Inactive panels are hidden; fullscreen temporarily hides the panel; closing it restores the page bounds.

`src/ui/devtools-bridge.js` supplies an in-process WebSocket-compatible transport to the frontend. Commands travel through the established IPC/event loop to WebView2's `CallDevToolsProtocolMethod` or its session-aware variant. Native protocol callbacks and event receivers return results to the frontend. No debugging port or network WebSocket server is required. Generation IDs reject late callbacks from closed/reopened panels.

The frontend's internal docking is disabled because Ventus owns the layout. The page context-menu handler retains the actual element (including open shadow-DOM targets). An Inspect request waits for the frontend's target and Elements model before invoking the CDP command-line `inspect()` helper. Closing the panel unregisters event receivers and disables the main enabled protocol domains.

## Frontend provenance and compatibility

The frontend is loaded from Microsoft's official HTTPS asset server at revision `af2ba23ae0ecf7084d900446fc59d5119dd5fd5e`. This matches the WebView2 runtime used for verification: Edge `156.0.4314.50`. `assets/devtools-events.json` contains 233 protocol event names from that runtime's `/json/protocol` schema, excluding malformed names. These event subscriptions include session IDs for child targets.

The first load requires internet access to `msedgedevtools.microsoft.com`; WebView2 caches assets in the browser profile. Offline availability is not guaranteed. The frontend is pinned, so a future runtime protocol change may require refreshing both the frontend revision and event list. Frontend navigation stays within that revision, and new-window requests are blocked so the virtual transport cannot open in an unrelated browser.

## Verification

Run `cargo test --locked` and `node --test scripts/test-devtools*.cjs scripts/test-window-resize.cjs`. The Rust tests cover panel geometry, scaling, fullscreen, and shortcut routing. JavaScript tests cover context-menu dispatch, keyboard commands, bridge transport, and fullscreen resize suppression.

Live checks also need to confirm that a first-open Inspect request selects the clicked tag, repeated inspection changes the selection, the frontend fills the panel, the picker selects a node, resizing changes page bounds, and fullscreen/closing restore bounds. Console and debugging features are supplied by the actual Microsoft frontend; WebView2 may reject protocol commands it does not support.
