# Tasks

## 1. Tokens, shell, and appearance control

- [x] 1.1 Replace the colour tokens in `src/web/app.css` with the semantic set (`--page`, `--panel`, `--panel-alt`, `--rule`, `--ink`, `--muted`, `--accent`, `--on-accent`, `--ok`, `--bad`, `--code-bg`, `--code-ink`, `--active-bg`, `--active-ink`, `--focus`, `--radius: 0`) in the light root block and both dark/light selector copies; verify `cargo test web::tests::assets_are_served_with_their_content_types_and_no_cache` still finds every palette selector.
- [x] 1.2 Rebuild the base layer: mono system stack at ~0.875rem, 1.6 UI line height, square 1px borders, no shadows, ~56rem shell, header with the `~ / memorija` brand, boxed active nav, and tabular numerals; verify no palette literal remains outside the token blocks (`rg -n '#[0-9a-fA-F]{3,8}' src/web/app.css`) and every screen still uses tokens only.
- [x] 1.3 Replace the three appearance chips in `src/web/index.html` with inline sun/moon/monitor SVG buttons keeping `data-theme-choice="light|dark|system"` and adding `aria-label`, `aria-pressed`, and `title`, plus the `THEME:` readout element; verify `cargo test web::tests::every_screen_address_serves_the_shell_with_a_tight_csp` passes once its assertions cover the labels and readout.
- [x] 1.4 Update the appearance logic in `src/web/app.js`: `markAppearance` also updates the readout from the effective appearance (root `data-theme`, else `matchMedia("(prefers-color-scheme: dark)")`) and one media-query listener keeps it true while System is active; verify in a browser that each choice switches at once, the readout names the effect, and flipping the OS appearance with System selected updates it without a reload.
- [x] 1.5 Extend the shell test in `src/web/mod.rs` to assert each appearance button's `aria-label` and the presence of the `THEME:` readout; verify with `cargo test web::tests::every_screen_address_serves_the_shell_with_a_tight_csp`.

## 2. Changes screen

- [x] 2.1 Rework the Changes markup and styles into the boxed row grid: decorative `▸` (`::before`), time, truncating path, right-aligned cause / `+N -M` / version, and `# date` headings with a trailing rule; verify with a long nested path at desktop and phone widths in both appearances that the path ellipsises and the right cluster stays aligned.
- [x] 2.2 Wrap both list filters in a `div.field` with the aria-hidden `/` glyph and add the `[press / to filter]` hint on the Changes intro line (hidden below 40rem); verify the field focuses on click, `:focus-within` accents the border, and the input keeps its accessible name.
- [x] 2.3 Add the guarded document `keydown` handler: `/` (with no Control, Alt, or Meta held, so shifted layouts work) focuses the current screen's filter and places the caret at the end, `Escape` blurs it without clearing; verify `/` works on Changes, is entered literally while the filter has focus, does nothing with Control/Alt/Meta held, and never fires on the Reader.
- [x] 2.4 Verify the Changes live behaviour is unchanged: with the feed scrolled away, a new pushed change still shows the "N new changes" affordance, and applying it keeps the filter text and caret; check both palettes and the empty/filter-empty states.

## 3. Files screen

- [x] 3.1 Rework the Files list into `:: DIRECTORY` headings with trailing rules and the row metadata as a right-aligned cluster, and add the file count to the intro line; verify the count matches what is listed, becomes `N of M files` while filtering, and updates when a pushed file appears or disappears.
- [x] 3.2 Verify the shared `/` shortcut also focuses the Files filter, `Escape` blurs it, and clearing the filter restores the full list without losing the count.

## 4. Reader

- [x] 4.1 Add `lines` (the line count of the content actually returned) to `FileResponse` in `src/web/mod.rs` and cover it in the API test for both the current content and a recorded version; verify with `cargo test web::tests` that the field is present and correct.
- [x] 4.2 Build the Reader file bar: back link and line count on top, `:: path` left and version tabs right (`v1 … vN` plus `HEAD` as real links, active one filled, strip scrolls and centres the active tab), a static `HEAD` badge without history, and the slim past-version line with "view latest"; verify a versioned file opens at each tab, a past version is marked and returns to latest, middle-click on a tab opens the stable URL, and a 30-version file keeps the path readable.
- [x] 4.3 Restyle the rendered markdown (h1 rule, mono headings, square accent bullets, boxed inline code, framed `pre`, bordered tables, blockquote rule) with a ~76ch prose measure; verify with a memory file containing a heading, list, code block, table, and quote in both appearances, and confirm raw HTML in a file still renders as text.
- [x] 4.4 Slim the live-update banner to the new shape and verify that a pushed version still offers "view vN" without replacing the open content, and that the bar marks the version actually shown instead of HEAD once HEAD moves.

## 5. Integration and polish

- [x] 5.1 Check the palette against contrast targets in both appearances (≥ 4.5:1 for body text and links, ≥ 3:1 for muted and status colours) and adjust tokens until they pass; verify with a contrast tool on the actual panel backgrounds.
- [x] 5.2 Exercise the responsive rules at 360, 768, and 1280 px: the header wraps with the theme label visible, list rows reflow to two lines, the Reader bar wraps tabs under the path, and nothing overflows horizontally.
- [x] 5.3 Run `cargo test` and manually walk `/`, `/changes`, `/files`, `/file`, an unknown address, and a legacy `/?path=` bookmark; verify the CSP header is byte-identical, the shell still serves with `no-cache`, and SSE keeps updating across screen switches.
- [x] 5.4 Re-read the design risks and confirm each is handled or explicitly accepted, especially long histories, long paths, icon-only accessibility (keyboard focus ring, pressed state), and the `▸` marker not presenting itself as a control.

## Workflow follow-up

- At archive, update `README.md`: the appearance paragraph (icon controls and `THEME:` readout), the `/` filter shortcut, and the terminal styling; add any developer-relevant token/asset note under Development.
- At archive, bump the crate version in `Cargo.toml` minor (`0.5.0` → `0.6.0`).
- Archive the change after review and verify the archived spec carries the modified appearance requirement and the added shortcut requirement.
