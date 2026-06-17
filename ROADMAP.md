# Roadmap

This roadmap is a guide, not a promise. Priorities may change as the browser stabilizes.

## Recently Done

- Download rows now open the saved local file when clicked, while action buttons still keep their own behavior.
- Network failure pages now render as a chrome-owned `neura://error` page instead of briefly showing the default WebView2 error page.
- Error-page refresh now retries the original failed URL instead of reloading a blank internal page.
- History now has a confirmation step before clearing everything, plus a cleaner search bar, time filters, and visible-link tools.
- Automatic reports no longer send session-start heartbeats and now focus on errors plus serious warning pages.
- Account settings now have a cleaner profile header and photo controls.

## Near Term

- Keep improving new tab reliability.
- Keep web content reliably clickable around chrome overlays and internal pages.
- Polish the remaining bookmarks flows.
- Improve settings persistence and defaults.
- Add clearer release packaging.

## Browser Quality

- Better navigation state.
- More complete download progress handling.
- Better page loading feedback.
- Stronger shortcut coverage.
- Keep hardening WebView and network failure states.

## AI

- Safer page-context sharing.
- Better provider setup and validation.
- Clearer AI sidebar states.
- Optional browser actions with confirmation.

## Platform

- Keep Windows as the main supported target first.
- Explore macOS and Linux only after the Windows shell is stable.
