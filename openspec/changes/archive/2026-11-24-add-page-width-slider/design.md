# Design

## Context

See `proposal.md` — Why. The viewer is four embedded assets served by `src/web/mod.rs` (`index.html`, `app.css`, `app.js`, and the render-blocking `theme.js` preference bootstrap), under a CSP pinned byte-for-byte by a test (`src/web/mod.rs:41`). There is no build step and no server-side rendering; every screen is drawn client-side from `fetch`/SSE data.

Constraints that shape this design:

- **The CSP forbids inline `style` attributes.** Width must come from `app.css` and a custom property written through CSSOM, which `style-src` does not block. No `style="…"` in markup, no injected `<style>`, no inline script.
- **No flash of the wrong width or appearance:** both stored values must be on `<html>` before the body paints, so the `<head>` bootstrap, not `app.js` at the end of `<body>`.
- **`body` is the shell:** `mode: max-width: 56rem; margin: 0 auto` (`src/web/app.css:106`) wraps header, lists, Reader and notices alike.
- **`40rem` is the layout breakpoint** (`src/web/app.css:552`): below it the row grids collapse to two lines, so the shell must not go narrower while the window is wider.
- **The Reader caps prose at `76ch`** (`src/web/app.css:472`), left-aligned inside its panel.
- **`app.js` rebuilds the whole screen subtree** on navigation, so any control placed inside `#screen` loses its listeners and its value on every screen change.

An earlier draft of this change put the width control in the header and deleted the `THEME:LIGHT` readout to make room. That is abandoned; D1 explains why.

## Goals / Non-Goals

**Goals:**

- One surface owns both settings, reachable from every screen, without competing with the header for space.
- The width applies continuously between a comfortable minimum and the full window, survives reloads without a visible reflow, and is visible while it is being changed.
- The list screens and rendered files use the extra width; prose line length stays readable.
- Appearance keeps its three choices, its pressed state and its remembered value while gaining visible labels.

**Non-Goals:**

- Discrete width presets, per-screen widths, or a keyboard shortcut for the width.
- Centring the reading measure (declined; prose stays left-aligned as today).
- Any server-side setting, new route beyond the renamed bootstrap, or API change; any automated visual test harness.
- Any setting beyond these two — the dialog is not a general preferences framework.

## Decisions

### D1: One settings dialog instead of controls in the chrome

Both settings are set-once-per-session. The header is at capacity, and the previous attempt to fit a control there had to delete the `THEME:` readout — a rare-use control paying for the removal of an always-visible one. A modal makes room on demand, gives the same home to both settings, and removes two problems the chrome placements had to design around: nothing floats over prose, and the width track gets the dialog's full width instead of a `~7rem` strip where a drag moved under a pixel per step.

A consequence worth stating: the header becomes navigation only — brand, three screen links, one button — which is a cleaner header than any of the chrome placements produced, and nothing hides at the `40rem` breakpoint any more.

Alternatives considered: a range slider in the header (the abandoned draft: cramped track, dead low end on small windows, needed the readout's slot); a floating pill in a viewport corner (occludes prose at 100 % width, and both corners are already spoken for — the browser paints hovered link targets bottom-left and overlay scrollbars bottom-right); a dedicated `/settings` screen (loses the live preview of the page behind it, adds a route and a history entry for a two-control panel).

### D2: A native modal `<dialog>` in the static shell

`<dialog>` opened with `showModal()` gives the top layer, modality, a focus trap, Escape and `::backdrop` natively, and is CSP-clean because all styling comes from `app.css`. A closed `<dialog>` is `display: none`, so it costs no layout and keeps its controls in the DOM — which is what lets `markAppearance()` and the shell test keep working unchanged in shape.

Two details it needs explicitly: `position: fixed; margin: auto` (the element sits in `<body>`, which is `max-width: 56rem; margin: 0 auto`, and the dialog must be centred on the viewport at any shell width), and `width: min(21rem, calc(100vw - 2rem))` so it fits a phone. Where `showModal` is missing, fall back to setting the `open` attribute.

The markup lives after `</main>`, beside the notice, not inside `#screen` — see the Context note about rebuilds.

Alternatives considered: a hand-built overlay (focus trap, `inert` background and Escape handling written by hand, more code and more places to get it wrong); the `popover` attribute (light-dismiss but no modality or focus trap, wrong semantics for a settings panel).

### D3: Appearance as three labelled segments in a fieldset

The existing three buttons move into a `fieldset` inside the dialog, gain visible text labels ("Light", "Dark", "System") and keep `data-theme-choice` and `aria-pressed`. The click handler and `markAppearance` are unchanged in substance — only the container moves — and the `fieldset`/`legend` takes over the naming the old group `aria-label` provided, while a `legend` is stronger semantics than an anonymous group.

With the dialog showing which segment is pressed, the `THEME:` readout is redundant rather than displaced, so it goes for a good reason this time. `effectiveAppearance()` and the `matchMedia` listener existed only to keep that readout true: the no-choice state is already handled by the stylesheet's `prefers-color-scheme` block, so both are deleted.

Alternatives considered: radio inputs (more idiomatic for an exclusive choice, but a larger rewrite of working code for no user-visible gain at three options); icons without words (what exists today, worse now that there is room to be explicit).

### D4: Live apply, no Save and no Cancel

Both settings write through as they are touched; Close, Escape and a backdrop click only dismiss. A Cancel would have to snapshot and restore two values, and there is nothing to confirm — the scrim is translucent, so the width change is visible behind the dialog as it happens. Persistence is per interaction, not per dialog session: the width is stored on the range's `change` event and the appearance on click, both already resilience-checked against a throwing store.

### D5: Percent of the window, clamped by a `rem` floor

The stored value is an integer percentage (40–100). `app.css` declares `--shell: 55` on `:root` and the shell becomes:

```css
body { max-width: clamp(40rem, calc(var(--shell) * 1vw), 100%); }
```

The value is unitless so CSSOM can write it straight from the bootstrap and the slider (`root.style.setProperty("--shell", value)`); a stale or hand-edited value is clamped to 40–100 on restore and falls back to the default on a parse failure. The `40rem` floor is what keeps the row grids intact when a small percentage meets a small window; the `100%` ceiling plus the body's `1rem` padding keeps the shell off the glass edges.

On a 1280 px laptop the bottom of the range maps to 512 px, which the floor lifts to 640 px. That was a real cost in the abandoned header draft, where it made part of the track inert; in a dialog the control sets a value rather than tracing a width, and the visible "N % of the window" readout plus the reflow behind the scrim make the floor obvious instead of mysterious.

Alternatives considered: a `rem` scale (a value like `140rem` means nothing to a user, and the top end cannot express "the whole window"); gutters (`--gutter: 1rem` with `max-width: calc(100% - 2 * var(--gutter))`) (equivalent at both ends, but the number then means wasted space rather than used space); no clamp (breaks the row grids below the breakpoint).

### D6: The `<head>` bootstrap becomes `prefs.js`

`src/web/theme.js` is renamed to `prefs.js` (route `/prefs.js`, `include_str!` constant, `index.html` script tag, asset test list): it now restores two preferences, so the appearance name would misdescribe it. It reads `memorija.width`, clamps it, and writes `--shell` through CSSOM after setting `data-theme`; on any storage error it leaves both defaults in place, exactly as it does today for the theme.

Alternatives considered: keeping the name (a lie in the file, comment and route); applying the width from `app.js` (the page paints at 55 % and jumps); an inline `<script>` (forbidden by CSP); a server setting (breaks the one-static-shell-for-every-browser model and writes state for a read-only viewer).

### D7: Prose keeps a measure, everything else fills

`.file-body > *` currently caps every child at `76ch`. That becomes a `--prose-measure: 120ch` cap on text blocks (headings, paragraphs, lists, quotes), with `min(120ch, 100%)` guarding a panel narrower than the measure, while `pre`, `table` and `img` are allowed the full panel width. Prose stays left-aligned; no centring.

At the default 55 % the cap rarely binds; near 100 % on a large display it does, which is the intended read: rows and code stretch, sentences stay around 120 characters.

Alternatives considered: removing the cap (mono at 200+ characters per line is a struggle to track); centring the capped block (declined); scaling the cap with the slider (two multipliers to reason about for little gain).

## Risks / Trade-offs

- **Appearance costs two extra clicks** (open the dialog, pick, close) where the trio was one → accepted: both settings share the visit, the header gains a single-purpose affordance instead of a dense trio, and a quick toggle beside the gear remains a small additive change that needs no requirement change.
- **A settings icon says "settings", not "appearance"** → accepted as convention; the button's accessible name states what it opens and the dialog's legend names the section, so nobody has to guess.
- **Settings dialogs accumulate settings** → this change ships exactly two; a third setting needs its own change, not a slot in this one.
- **A dark scrim over a dimmed page makes the width change less legible** → keep the scrim light enough to read the reflow, and verify at 40, 55 and 100 that the page behind visibly moves.
- **`<dialog>` defaults differ between engines** (border, padding, max sizes, backdrop colour) → reset `border`, `padding`, `max-width`, `max-height`, `color` and the backdrop in `app.css`, and verify in Safari and Firefox.
- **A stale or hand-edited `memorija.width`** → clamp to 40–100 on restore and fall back to the default on a parse failure.
- **A unitless range announces a bare number** → give the input `aria-valuetext` naming the percentage of the window, and show the same text visibly beside it.
- **No at-a-glance width** once the dialog closes → accepted: the value is shown while it is being set, and the layout itself is the durable feedback.
- **Losing the readout removes the only textual statement of the appearance in effect** → the pressed labelled segment carries it, and the dialog is one click away.
- **The width value must not appear in any server response** → it lives only in `localStorage`; the shell test asserts the HTML, not the choice.

## Migration Plan

No persisted data and no schema: an absent `memorija.width` yields the default 55 %, and a stale key from a build without the control is simply never read. Rollback is reverting the commit; the stale key is ignored by the old build. `README.md` and the `web-ui` spec are updated in the same change, and `Cargo.toml` bumps minor (`0.6.0` → `0.7.0`) at archive, per `AGENTS.md`.
