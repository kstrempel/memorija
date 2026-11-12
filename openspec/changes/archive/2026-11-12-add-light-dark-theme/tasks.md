# Tasks

## 1. Appearance tokens in the stylesheet

- [x] 1.1 In `src/web/app.css`, add the `--fg`, `--surface`, `--raised`, and `--shadow` tokens to `:root` and replace every hard-coded colour in component rules (`body`, `.chip`, `.new-changes`, `.reader-title`) with a token; verify `grep -nE '#[0-9a-fA-F]{3,6}|rgba' src/web/app.css` lists colours only inside token declaration blocks.
- [x] 1.2 Add the dark token block under `@media (prefers-color-scheme: dark)` scoped to `:root:not([data-theme])`, the `:root[data-theme="dark"]` and `:root[data-theme="light"]` blocks with the same values, and `color-scheme: light` / `dark` in each; verify `cargo test --lib web` passes and that the media-query and attribute selectors are present as designed.
- [x] 1.3 Extend `src/web/mod.rs`'s asset test to assert `app.css` contains `prefers-color-scheme`, `[data-theme="dark"]`, and `color-scheme`; verify the test fails if either selector is removed and passes with the change.

## 2. Theme bootstrap asset

- [x] 2.1 Create `src/web/theme.js`: read `localStorage["memorija.theme"]` inside a `try`/`catch`, accept only `"light"` or `"dark"`, set or remove `document.documentElement.dataset.theme` accordingly; verify the file runs without error in a browser console context and treats an unknown value as no choice.
- [x] 2.2 Embed `theme.js` in `src/web/mod.rs` with `include_str!` and serve it at `GET /theme.js` as `text/javascript` with `Cache-Control: no-cache`; verify the asset test covers `/theme.js` and returns the expected header values.
- [x] 2.3 Add `<script src="/theme.js"></script>` to the `<head>` of `src/web/index.html` before the stylesheet is used by the body; verify the shell test asserts `/theme.js` appears in the document and that the CSP header is unchanged.
- [x] 2.4 In a running viewer, set `localStorage.setItem("memorija.theme", "dark")` on a light-OS browser, reload, and verify the very first paint is dark with no light flash; then remove the key and verify the page follows the system appearance.

## 3. Header control and client wiring

- [x] 3.1 Add the `appearance` group to `src/web/index.html`'s header with three `button` elements (`data-theme-choice="light|dark|system"`, `aria-pressed`), reusing `.chip`; verify the shell test asserts the group label and all three choices are served, the buttons are reachable by keyboard, and the header wraps them on a narrow viewport.
- [x] 3.2 In `src/web/app.js`, compute the effective choice (stored value, else `"system"`), mark `aria-pressed` on the matching button, and attach one delegated click handler that stores the choice (removing the key for System), sets or removes `data-theme`, and re-marks the buttons without re-rendering the screen; verify each choice applies immediately, keeps the current screen and its filter text, and survives a reload.
- [x] 3.3 Wrap storage writes so a throwing or missing `localStorage` cannot break the page; verify with storage blocked that the control still switches appearance for the session, no console error appears, and a reload returns to the system preference.
- [x] 3.4 Update the web viewer section of `README.md` to describe the appearance control, the System default, the browser-local storage, and the per-origin behaviour; verify the documented behaviour matches the running viewer and the CLI usage block is unchanged.

## 4. Integration checks

- [x] 4.1 Run `cargo test` and verify the full suite passes, including the shell, asset, and `tests/web_posture.rs` checks.
- [x] 4.2 With `memorija web` running, check Changes, Files, Reader (current and past version), not-found, and empty states in Light and Dark: verify no light-only patch remains, muted text and the `+N -M` colours are legible, and markdown code blocks and tables match the appearance.
- [x] 4.3 Switch the OS appearance while the page is open with no explicit choice: verify the open screen changes without a reload, and that an explicit Light or Dark choice ignores the OS switch.
- [x] 4.4 Verify the viewer stays read-only and the policy unchanged: the CSP string test still passes byte-identically, `git diff src/web/mod.rs` shows only the `theme.js` embed and route, and `memorija web` creates no file under the memory root or version store.

## Workflow follow-up

- Bump the crate version in `Cargo.toml` (minor, per AGENTS.md) and review the README additions before archiving.
- Archive the change with `openspec archive` once the review is done.
