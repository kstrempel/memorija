# Proposal

## Why

The web viewer ships a single light appearance with hard-coded colours, so it is
uncomfortable to use in a dark environment and ignores the operating system's
appearance preference. People read memory files in the evening and in editors
that are already dark; a viewer that flashes a white page is the odd one out.

## What Changes

- The viewer gains a dark appearance alongside the current light one, built from
  the same stylesheet so every screen (Changes, Files, Reader, not-found) is
  covered.
- On first visit the appearance follows the operating system / browser
  preference, and it keeps following it live when the system switches while a
  screen is open — no reload needed.
- A compact appearance control in the shared header offers **Light**, **Dark**,
  and **System**. The explicit choice is remembered in the browser and used on
  later visits; choosing **System** returns to following the OS.
- The choice is a per-browser setting only: nothing is written to the memory
  root, the version store, or the server, and the viewer stays read-only.
- No change to addresses, endpoints, the Content-Security-Policy, or the CLI.

## Capabilities

### New Capabilities

None: the viewer's behaviour is already owned by `web-ui`.

### Modified Capabilities

- `web-ui`: adds requirements for a light and dark appearance, a default that
  follows the system preference, and a remembered explicit choice.

## Impact

- `src/web/app.css` — theme tokens for both appearances, dark values, and
  `color-scheme` so form controls and scrollbars match.
- `src/web/index.html` — the header gains the appearance control (static markup,
  no inline script).
- `src/web/app.js` — reads the stored choice, applies it to the document,
  listens for system-preference changes, and wires the control to
  `localStorage`.
- `src/web/mod.rs` — no functional change; the embedded assets are unchanged in
  shape. Its asset/shell tests gain assertions for the theme markup and tokens.
- `tests/web_posture.rs` — may assert the control is served.
- `README.md` — document the appearance control in the web viewer section.
- No new dependencies, no front-end build step, no persisted server state.
