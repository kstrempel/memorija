# Design

## Context

See `proposal.md` — Why. The viewer is three embedded assets served by `src/web/mod.rs` (`index.html`, `app.css`, `app.js`) plus the `theme.js` appearance bootstrap, `include_str!`ed with `Cache-Control: no-cache` under a CSP that forbids inline script and style and allows only same-origin assets (`default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; …`). There is no front-end build step, no framework, and no server-side rendering.

The current sheet is a conventional light card UI: sans-serif body at `52rem`, rounded 6px boxes, text chips for the appearance, with the light and dark palettes duplicated across three selectors (`:root`, the `prefers-color-scheme` media query on `:root:not([data-theme])`, and `:root[data-theme="…"]`) so the stored choice paints before the body. `theme.js` is a parser-blocking `<head>` script; `app.js` renders every screen client-side from `fetch`/SSE data. The version store keeps each record's full content, and `/api/file` already returns history with per-version `+added/-removed`, so the reskin needs no new diff capability.

The agreed mockups are 2x captures of a ~950 CSS px viewport; at ~900 CSS px the shell is close to today's `52rem`. What changes is type, density, rules and geometry — not the shell width.

Constraints that shape the design:

- CSP stays byte-identical; no inline `style`, script, or event handler. Inline SVG markup is allowed; a bundled font would need a new asset route and `font-src 'self'`.
- The viewer stays read-only; nothing about appearance or the shortcut reaches the server.
- Existing shell tests pin the three `data-theme-choice` attributes, the `aria-label="Appearance"` group, the asset routes, and the palette selectors in `app.css`; behaviour tests pin the `/api/file` JSON.
- All addresses, routing, filters-in-URL, SSE, version browsing, and path-safety stay as they are.

## Goals / Non-Goals

**Goals:**

- One stylesheet and the existing scripts express the terminal idiom on every screen and empty/not-found state, in both appearances, without losing any current behaviour.
- The appearance control becomes three icon buttons with accessible names and a `THEME:X` status readout that reflects the appearance in effect.
- Reader version navigation stays URL-addressable and copy-linkable while living in the file bar.
- The `/` shortcut and its hint are real behaviour, not decoration.

**Non-Goals:**

- Expandable rows, inline diffs, or any new endpoint.
- A version dropdown or menu; version navigation stays links.
- Custom/bundled fonts, syntax highlighting, images in markdown, or a front-end build step.
- Showing the real memory root in the header; the prompt brand stays static.
- Automated visual or browser tests; there is no harness, and this change does not add one.

## Decisions

### D1: Semantic tokens, same three-block palette structure

`app.css` moves to semantic tokens: `--page`, `--panel`, `--panel-alt`, `--rule`, `--ink`, `--muted`, `--accent`, `--on-accent`, `--ok`, `--bad`, `--code-bg`, `--code-ink`, `--active-bg`, `--active-ink`, `--focus`, plus `--radius: 0`. The light palette stays on `:root` and is repeated for `[data-theme="light"]`; the dark palette stays under `prefers-color-scheme` on `:root:not([data-theme])` and is repeated for `[data-theme="dark"]`, in the same adjacent, commented form as today, with `color-scheme` set per block. Light panels are shaded slightly darker than the page; dark panels are lifted. This keeps the no-flash bootstrap and the "System changes apply live with no listener" property.

Alternatives considered: `light-dark()` (uneven browser support here, silent fallback to light), two stylesheets (duplicate rules, still flashes), `filter: invert()` (destroys accent and status colours).

### D2: System monospace, capped reading measure

`body` uses a system monospace stack (`ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`) at ~`0.875rem` with a `1.6` UI line height. Uppercase micro-labels (group headings, `THEME:`, `HEAD`) use letter-spacing; data columns use `font-variant-numeric: tabular-nums`. The shell stays around `56rem` with `1rem` side padding. Reader prose is capped at roughly `76ch` inside its framed panel at `1.75` line height, matching what the mockup shows, so long paragraphs do not run the full panel width. Geometry is square: 1px rules, no radius, no shadow.

Alternatives considered: a bundled terminal font (binary size, a new route, `font-src`), sans-serif prose (breaks the idiom; the mockup is mono), a full-bleed shell (the capture is ~900px; wider worsens the reading measure).

### D3: Icon appearance control with a status readout

`index.html` keeps the `role="group" aria-label="Appearance"` wrapper and the three `data-theme-choice` values, but each child becomes a button holding an inline 16px SVG (sun, moon, monitor) and carrying `aria-label="Light|Dark|System"`, `aria-pressed`, and a native `title`. Icons are `aria-hidden="true"` and `focusable="false"`; focus uses `:focus-visible` with the accent outline. A `THEME:X` readout sits beside the group with `aria-hidden="true"` (the pressed state already conveys the choice).

The readout names the **appearance in effect**: with System selected on a dark OS it reads `THEME:DARK`. `app.js` derives it from the root's `data-theme` when set, otherwise from `matchMedia("(prefers-color-scheme: dark)")`, and attaches one change listener to keep it true while System is active. The active segment uses `--active-bg`/`--active-ink` (pale blue fill with dark glyph in dark, solid accent with light glyph in light), matching the mockup.

Alternatives considered: keeping text chips (wider, not the mockup), a native `<select>` (loses one-click visibility and the pressed trio), showing the choice instead of the effective appearance (simpler, no listener, but says less and contradicts the mockup's `THEME:DARK`).

### D4: CSS grid rows, decorative marker, terminal headings

Each list row stays a single `<a>` wrapping a grid: marker, time, path (`minmax(0, 1fr)`, ellipsis), then a right-aligned cluster for cause, `+N`, `-M`, version (Files: lines, version, date). The `▸` is CSS `::before` content on the row, `aria-hidden` by construction, and is not an affordance — clicking anywhere still opens the Reader, so `▸` must not look like a control (no hover state of its own). Day headings become a flex line: an accent `#`, the date, and a flexible hairline rule; Files groups use `::` and the directory label with the same trailing rule. The Files intro becomes a flex row with the file count on the right.

At `max-width: 40rem` the row grid collapses to two lines (marker + time + path; then the right cluster, right-aligned) and the Files/Reader bars wrap.

Alternatives considered: a disclosure button with inline diff (explicitly out of scope; also invalid inside the row anchor), a table (less liquid), separate columns per breakpoint without reflow (illegible on phones).

### D5: The filter is a wrapper plus a guarded document shortcut

Both list screens render a `div.field` containing an `aria-hidden` `/` glyph and the existing `input[type=search]`; the input keeps its `aria-label`, and `:focus-within` accents the field border. The Changes screen's hint (`[press / to filter]`, with a boxed keycap) sits on the intro line, right-aligned, hidden at `max-width: 40rem`; the Files screen relies on the boxed `/` prompt, matching the mockup, while the shortcut works on both.

One document-level `keydown` handler on the list screens implements the shortcut: fire only on `/` with no Control, Alt, or Meta held (Shift is allowed, because `/` needs it on some layouts), only when `event.target` is not an input/textarea/select and not inside `[contenteditable]`, and only when not `defaultPrevented`; then `preventDefault()`, focus the filter, and place the caret at the end. An `Escape` keydown on the filter blurs it without clearing. Screen changes re-attach nothing: the handler is installed once and asks the current state for its filter, exactly like the delegated link handler.

Alternatives considered: `accesskey` (browser-dependent and announces differently), relying on the placeholder alone (the mockup promises the key), intercepting keys globally (breaks typing).

### D6: Reader bar with version tabs

The Reader keeps its back link and gains the line count on the right. The file bar is a flex row: `:: path` on the left (`min-width: 0`, ellipsis) and a version tab group on the right. With history the group is a horizontal scroller of real anchors — `v1 … vN` plus `HEAD`, where `HEAD` is the current-content address with no `version` parameter. The shown version is filled (`--active-bg`); the strip scrolls, and on render the active tab is scrolled into the centre, reusing today's strip logic. Without history the bar shows `HEAD` as a static `<span>` badge. A past version adds a slim line under the bar ("viewing v2 of v3 — view latest"), replacing today's larger `.past-version` card; the "This file was updated" live banner keeps the same slim shape. If a live update makes HEAD stale, the bar marks the version actually on screen instead of HEAD while the banner offers the newer one. Below `40rem` the bar wraps: path first, tabs beneath it.

Alternatives considered: the version rail below the bar (the other explored option; robust but adds a row and departs from the mockup), a `HEAD ▾` dropdown (breaks copy-link and middle-click, which the spec's stable addresses rely on).

### D7: `lines` is computed for the version actually returned

`FileResponse` gains `lines: usize`, computed server-side from the same content the response renders (current file or the chosen record), so a direct Reader load never depends on the live file index arriving first. The client shows it beside the back link. This is an additive JSON field; no new route.

Alternatives considered: reusing `state.files` (empty/racy on a direct load), counting lines in JS from rendered HTML (wrong once markdown is rendered), dropping the count (it is in the mockup).

### D8: Markdown keeps its HTML, gains terminal styling

`markdown.rs` does not change. `app.css` styles the existing tags: mono headings with an `h1` bottom rule, square accent bullets, boxed inline code with `--code-bg`/`--code-ink`, framed `pre`, bordered tables, blockquote rule, and link styling. Prose sits in a framed panel with `max-width: 76ch`. No HTML passthrough and no `img` allowance are added; the CSP still blocks remote images exactly as before.

### D9: Tests, docs, and version

The shell test keeps its current assertions and adds: each appearance button has an `aria-label`, and the shell carries the `THEME:` readout (a data attribute or element, not its text). The API test covers `lines` for both the current content and a past version. `app.css` selector assertions stay. The README's appearance paragraph and the screen descriptions gain the icons/readout and the `/` shortcut; `Cargo.toml` bumps minor (`0.5.0` → `0.6.0`) at archive, per `AGENTS.md`. Visual verification is manual: both appearances on all screens at desktop and phone widths, plus a file with many versions.

## Risks / Trade-offs

- **Muted text on the shaded light rows can fall below 4.5:1** → pick the muted token against `--panel`, not the page; target ≥ 4.5:1 for body/link and ≥ 3:1 for muted/status, checked with a contrast tool during implementation.
- **Long histories crowd the Reader bar** → the path truncates first and the tab group scrolls; verify with a 30+ version file and with a path that is itself long.
- **Mono prose reads worse than a serif/sans at the same size** → cap at ~76ch, use 1.75 line height, and keep body copy at 14px rather than shrinking it further.
- **Icon-only controls can lose meaning for assistive tech** → every button has an `aria-label` and `aria-pressed`, the readout is `aria-hidden`, and focus is visibly outlined; verify with keyboard-only navigation.
- **The status label can drift from the OS while System is active** → one `matchMedia` change listener updates it; test by flipping the OS appearance with System selected.
- **`▸` can be mistaken for a disclosure** → it has no hover or rotation and the row's hover feedback covers the whole row; the README says rows open the Reader.
- **Search inputs render browser-specific controls** → keep `type=search` and normalize `appearance`/border/padding so both appearances are consistent.
- **The mockups show only a file without history** → D6 is an agreed extrapolation; it is the largest piece of new UI to eyeball in both appearances.
- **Rows are grid anchors with long paths** → `minmax(0, 1fr)` plus ellipsis on the path; test a deep nested path at phone width.

## Migration Plan

Purely additive for users: same binary, same `web` subcommand, same addresses, same CSP, same `localStorage` key and no-flash behaviour. The only behavioural additions are the `/` shortcut and the visible file count; everything else is presentation. Rollback is reverting the commit — no persisted data, no schema, no migration. The version bump lands with the archive.
