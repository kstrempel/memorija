# Tasks

## 1. Preference bootstrap

- [x] 1.1 Rename `src/web/theme.js` to `src/web/prefs.js`, updating the `include_str!` constant, the `/prefs.js` route and handler, the `<script>` tag in `index.html`, and the asset test list; verify `cargo test web::` passes and `cargo run -- web --no-open` serves `/prefs.js` while `/theme.js` is no longer routed.
- [x] 1.2 In `prefs.js`, read `memorija.width`, clamp a parsed value to 40–100, and write it with `root.style.setProperty("--shell", value)` after the `data-theme` handling, leaving both defaults untouched on any storage or parse failure; verify by setting two different values in the browser's `localStorage`, reloading each time, and confirming the page paints at the stored width with no visible jump and `--shell` on `<html>` in the inspector.

## 2. Shell width and reading measure

- [x] 2.1 Declare `--shell: 55` on `:root` and replace the shell declaration with `max-width: clamp(40rem, calc(var(--shell) * 1vw), 100%)`; verify with the inspector at 512, 1024 and 2560 px viewport widths that the change and file rows keep their columns, that the shell never exceeds the window minus its `1rem` gutters, and that a `--shell` of 40 on a small laptop stops at 640 px.
- [x] 2.2 Replace the blanket `.file-body > *` cap with a `120ch` measure on prose blocks and let `pre`, `table` and `img` use the panel width; verify a file with a heading, list, quote, code block and table renders with bounded paragraphs and full-width code/tables at both 55 and 100, in both appearances.

## 3. Settings trigger and dialog

- [x] 3.1 Replace the header's appearance wrapper (`#theme-label` and `.theme-control`) with a single settings button carrying an accessible name, and add a `<dialog>` after `</main>` — outside `#screen` — with a heading, an Appearance section, a Page width section and a Close button; extend the shell test to assert the settings button and both controls inside the dialog on every screen, and that the dialog is closed without JavaScript; verify with `cargo test web::`.
- [x] 3.2 Style the dialog and its backdrop in `app.css`: explicit `position: fixed; margin: auto`, `width: min(21rem, calc(100vw - 2rem))`, square 1px `--rule-strong` border on `--panel`, border/padding/max-size resets, a translucent scrim, and the section labels in the existing uppercase micro-label style; verify in Safari and Firefox at 1024, 768 and 400 px that the dialog fits, stays centred on the viewport at `--shell` 40 and 100, and is legible over the scrim in both appearances.
- [x] 3.3 Wire the dialog in `app.js`: the settings button opens it with `showModal()` (falling back to the `open` attribute where the method is unavailable), and the Close button, Escape, and a click on the backdrop dismiss it and return focus to the settings button; verify keyboard-only — Tab to the button, Enter, Tab through both controls, Escape — that focus lands back on the button and the page behind did not scroll.

## 4. Appearance inside the dialog

- [x] 4.1 Move the three appearance buttons into a `fieldset` with a `legend` in the dialog, adding visible text labels while keeping `data-theme-choice` and `aria-pressed`; delete `#theme-label` and the `.theme-label` rule; extend the shell test to assert the legend, the three labels, the pressed state and the absence of `THEME:`; verify with `cargo test web::` and that clicking a segment marks it pressed.
- [x] 4.2 Remove `effectiveAppearance()` and the label's `matchMedia` listener from `app.js`; verify with System selected that flipping the OS appearance still switches the page with no reload, that System stays the pressed segment, and that Light or Dark still wins over the OS choice.

## 5. Page width control

- [x] 5.1 Add the range input inside the dialog's Page width section — `min="40" max="100" step="1"`, an accessible name, `aria-valuetext` naming the percentage of the window, a visible "N % of the window" value and 40 %/100 % end labels — and assert it, and its placement inside the dialog, in the shell test; verify with `cargo test web::`.
- [x] 5.2 Style the range for WebKit and Gecko inside the dialog at its full width (1px `--rule-strong` track on `--panel`, square `--accent` thumb, `:focus-visible` accent outline on the control); verify in Safari and Firefox, in both appearances, that track and thumb match and the focus outline is visible.
- [x] 5.3 Handle `input` and `change` in `app.js`: apply through `--shell` on every `input`, persist `memorija.width` on `change` while swallowing storage errors, and never re-render the screen; verify with a filtered Changes screen and an open Reader file behind the dialog that filter text, listed rows, open file and version survive the drag, and that a reload restores the width.
- [x] 5.4 Rewrite the README's appearance paragraph as a settings paragraph: both controls in one dialog, applied immediately, remembered per browser, width from a comfortable minimum to the full window; verify the described behaviour against `memorija web --no-open` screen by screen.

## 6. Integration pass

- [x] 6.1 Run `cargo test` and walk the Changes, Files, Reader, empty and not-found screens at phone, laptop and large-display widths with `--shell` at 40, 55 and 100 in both appearances; verify no screen overflows horizontally, no row loses a column, the dialog fits at 400 px, and the CSP header is unchanged.

## Workflow follow-up

- Bump `Cargo.toml` from `0.6.0` to `0.7.0`, fold the README notes into the Usage/Features and Development sections as `AGENTS.md` requires, then archive the change.
- Verify the archived `openspec/specs/web-ui/spec.md` carries the amended appearance requirement and the two added requirements.
