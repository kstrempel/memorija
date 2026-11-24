# Proposal

## Why

The viewer's shell is pinned at `56rem` (about 896 px), so on a large display the change and file rows are cramped while half the window sits empty, and how wide is comfortable differs per screen and per person. The header has no room left for another control: putting a width control there costs the `THEME:LIGHT` readout its place, so a setting touched twice a session displaces one that is read all the time.

## What Changes

- Replace the header's three-icon appearance control and its `THEME:` readout with a single settings icon. The header becomes brand, screen links, and that one button.
- Open a centered modal settings dialog from it, holding both of the viewer's settings: the appearance choice (Light / Dark / System) and a page-width slider spanning 40 % to 100 % of the window, with a `40rem` floor, defaulting to about 55 %.
- Apply both settings live while the dialog is open — no Save, no Cancel, only Close — and remember each per browser in `localStorage`, restored in the `<head>` bootstrap before the first paint so the page never reflows after paint.
- Keep Reader prose at a readable measure (a soft cap near 120ch, left-aligned as today); change and file rows, code blocks and tables take the full shell width.
- Reconcile spec, tests and README with the new settings surface and the removed readout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the appearance requirement moves the three choices into the settings dialog with visible labels and drops the status readout; two requirements are added for the settings dialog itself and for the adjustable, remembered page width.

## Impact

- `src/web/index.html` — the header keeps brand, screens and one settings button; a `<dialog>` with both controls joins the static shell after `</main>`.
- `src/web/app.css` — the shell `max-width` becomes a `clamp` over `--shell`; dialog, backdrop, section labels and slider styling; the `.theme-label` rule goes; prose measure.
- `src/web/app.js` — dialog open/close; width handling and persistence; the readout update, `effectiveAppearance()` and the label's `matchMedia` listener go.
- `src/web/prefs.js` — renamed from `theme.js`; restores both preferences before paint.
- `src/web/mod.rs` — the `prefs.js` route and constant; shell test assertions change from the icon trio and readout to the settings button, the dialog and its two controls.
- `README.md` — the appearance paragraph becomes a settings-and-width paragraph.
- No API or JSON changes and no server-side state; the CSP stays byte-identical and the viewer stays read-only.
