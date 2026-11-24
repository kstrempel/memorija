# Proposal

## Why

The web viewer works, but its generic light card layout — sans-serif type, rounded 6px boxes, text chips for the appearance — does not express what memorija is: a terminal-first, file-based memory tool for coding agents. The agreed mockups restyle every screen in the terminal idiom the tool belongs to (monospace throughout, square hairline boxes, `#` and `::` section markers, prompt-shaped brand) and replace the three text appearance chips with a compact status readout plus sun/moon/monitor icons, without giving up any of the viewer's current behaviour.

## What Changes

- **Shared visual language**: monospace UI (~14px), square 1px boxes, no shadows or radii, hairline rules, terminal section markers; a rewritten token palette for both appearances (light panels shaded darker than the page, dark panels lifted). The shell keeps roughly today's width.
- **Header**: prompt brand `~ / memorija`, the current screen marked by a boxed outline, and the appearance control becomes a `THEME:LIGHT|DARK` readout beside three icon buttons (sun, moon, monitor) with accessible names. The stored choice, its `localStorage` key, the three options, and the pre-paint bootstrap are unchanged.
- **Changes screen**: `# YYYY-MM-DD` day headings with a trailing rule, rows in a boxed group with a decorative `▸` marker, time/path on the left and cause / `+N -M` / version right-aligned, `/`-prefixed filter with a `[press / to filter]` hint, and `/` as a keyboard shortcut that focuses the filter.
- **Files screen**: `:: DIRECTORY` group headings with a rule, a right-aligned count of listed files, and the existing line/version/date metadata as a right-hand cluster.
- **Reader**: a file bar showing `:: path` with the line count above it and version tabs on the right (`v1 … HEAD`); recorded versions stay real links, the shown one is filled, and a past version keeps a slim "viewing v2 of v3 — view latest" line. Files without history show `HEAD` as a static badge.
- **Small data addition**: `/api/file` gains a `lines` count for the content actually returned (the chosen version), so the Reader's count never depends on the live file index.
- Out of scope: expandable rows or inline diffs, a version dropdown, new endpoints, CSP changes, address/ routing changes, and any change to what the viewer can modify (it still cannot).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the appearance-control requirement now describes three icon controls with accessible names and a visible status readout instead of named text chips; the filter requirement gains the `/` focus shortcut (and the hint that announces it) for Changes and Files. All other requirements — addresses, filters, live updates, version browsing, read-only, palettes — are unchanged and still must hold.

## Impact

- `src/web/index.html`, `src/web/app.css`, `src/web/app.js`: restructured header, terminal styling, row/toolbar/Reader markup, `/` shortcut, Files count, `THEME:` readout.
- `src/web/mod.rs`: `lines` in the file response, plus test updates for the icon control and the new field. `theme.js`, `markdown.rs`, the router, SSE, and the CSP string stay as they are.
- `README.md`: the appearance paragraph and the new shortcut; `Cargo.toml` version bump on archive.
- No dependency or build-step change; a system monospace stack avoids adding a font asset.
