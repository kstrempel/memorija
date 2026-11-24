# Proposal

## Why

Memory files frequently carry agent-generated Mermaid diagrams — architecture notes, plans, sequence sketches — but the Reader shows them as a mono code block, so a human reading a memory has to copy the source into some other tool to see the shape of it. A diagram that is nothing but text on the page is a large part of why the file was written, and the viewer already renders the rest of its markdown.

## What Changes

- Render ` ```mermaid ` fenced code blocks in the Reader as SVG diagrams, in the current content and in every recorded version, keeping every other code block byte-for-byte as it is today.
- Bundle a pinned Mermaid build in the binary and serve it from the viewer's own origin (`/mermaid.js`), loaded only when a rendered file actually contains a diagram; the viewer keeps working offline and makes no third-party request.
- Relax the viewer CSP for styles only (`style-src 'self' 'unsafe-inline'`), which Mermaid's generated SVG requires; `script-src` and `connect-src` stay same-origin, and no inline script can run. This is a deliberate, documented posture change, not an oversight.
- Render diagrams with Mermaid's strict security level, SVG text labels rather than HTML labels, click handlers off, and no external resources.
- Follow the light/dark appearance: diagrams are drawn in a matching Mermaid theme and redrawn when the appearance changes, without rebuilding the Reader or losing the selected version or scroll position.
- Fall back to the original source block, unchanged and readable, when a diagram is invalid or Mermaid cannot be loaded; a broken diagram never becomes an error page.
- Keep wide diagrams inside the page: they scale down to the reading measure and scroll sideways within their own frame instead of widening the shell or overflowing it.
- Reconcile the spec, the CSP shell test, the README and the crate version with the new capability.

Out of scope: exporting or downloading a diagram, pan/zoom controls, clickable diagram nodes or links, `.mmd` files as a separate screen, client-side editing, and diagram rendering anywhere outside the Reader's rendered markdown.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the Reader's rendered-markdown behaviour gains Mermaid diagrams — when a fenced block is drawn as a diagram, how invalid diagrams and missing script degrade, how diagrams follow the appearance, and the escape/no-network guarantee that replaces the current "no inline style at all" posture.

## Impact

- `src/web/vendor/` — new: a pinned `mermaid.min.js` (plus its license notice), embedded with `include_str!`; the binary grows by roughly the size of that file.
- `src/web/mod.rs` — a `/mermaid.js` asset route, the relaxed `CSP` constant, and the shell test's CSP assertions split into "scripts stay strict" and "styles may be inline".
- `src/web/app.js` — diagram discovery after the Reader renders, lazy script loading, render/error handling, appearance-driven redraw, and per-diagram unique ids.
- `src/web/app.css` — diagram figure styling, overflow and scaling inside the existing prose measure, both appearances.
- `src/web/markdown.rs` — no behavioural change expected; fenced code keeps its `language-mermaid` class, which is the hook the client uses. A test pins that class so the contract cannot drift silently.
- `src/web/index.html` — no new markup; diagrams are created per rendered file.
- `README.md` — developer notes for the vendored bundle, the CSP exception and why it exists, plus the Reader description.
- `Cargo.toml` — minor version bump at archive, per `AGENTS.md`.
- No API, JSON, store, or address changes; the viewer stays read-only.
