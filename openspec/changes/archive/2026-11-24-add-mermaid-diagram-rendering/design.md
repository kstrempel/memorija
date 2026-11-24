# Design

## Context

See `proposal.md` — Why.

The Reader is the only screen that renders file content. `src/web/markdown.rs` turns a memory file's markdown into HTML with pulldown-cmark, rewriting raw HTML events to text so `push_html` escapes them; the server returns that HTML in `/api/file` and `app.js` assigns it once (`body.innerHTML = data.html` in `renderReaderData`, the existing trust boundary). Fenced blocks already come out as `<pre><code class="language-mermaid">…</code></pre>` with the source escaped — the hook a diagram renderer needs — and nothing in the client inspects code blocks today.

Constraints that shape the approach:

- **CSP.** `src/web/mod.rs` sends `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'` on the shell, and `every_screen_address_serves_the_shell_with_a_tight_csp` asserts that string byte-for-byte and rejects any `unsafe-inline`. Affected clients get no inline script, no inline style, no third-party origin.
- **No build step.** `index.html`, `app.css`, `prefs.js`, `app.js` are `include_str!`ed into the binary and served with `Cache-Control: no-cache`; `build.rs` only stamps the build date. The app is one local process with no network dependency, so the viewer must work offline.
- **No JS test harness.** `src/web/mod.rs` has Rust tests over the shell and the JSON endpoints; `tests/web_posture.rs` covers binding; client behaviour has always been verified manually (the archived designs say so explicitly).
- **Appearance plumbing.** `prefs.js` puts a stored `data-theme` on `<html>` before paint; `app.js`'s `applyAppearance` sets or removes it on click; with no stored choice the stylesheet follows `prefers-color-scheme` through its media query, and nothing in `app.js` listens for a system flip.
- **Reader lifecycle.** `renderReaderData` builds the body once per load/version; live updates deliberately do **not** replace it (they show an "updated — view vN" banner). So nothing today re-renders content without a new navigation.
- **Reading measure.** `app.css` gives the shell a `clamp(40rem, calc(var(--shell) * 1vw), 100%)` width and prose a `120ch` measure; a wide diagram must not reopen the horizontal page scroll the width change removed.

Verified while planning: Mermaid's current dist (`12.3.0`, MIT, ~5.5 MB minified UMD) assigns `globalThis["mermaid"]` and contains no `eval(` or `new Function(` — so `script-src 'self'` alone can run it, and only style injection is blocked by the current policy.

## Goals / Non-Goals

**Goals:**

- One vendored, pinned Mermaid bundle served from the viewer's origin; no third-party request, no build step, offline drawing.
- Diagrams drawn from the same server-rendered HTML the Reader already trusts, with the source block as the always-available fallback.
- A CSP that is exactly as strict as before *except* where drawing requires inline style, and a test that pins that distinction rather than the old byte string.
- Diagrams that look like they belong to the terminal idiom, in both appearances, without disturbing the file, version, scroll position, or the open settings dialog.
- No new endpoint, no new address, no store or API change.

**Non-Goals:**

- Pan/zoom, export (SVG/PNG), copy-source buttons, clickable nodes, `click`/`href` directives, or diagram editing.
- Rendering raw HTML `<div class="mermaid">` blocks: raw HTML is escaped by `markdown.rs` and stays escaped. Only fenced ` ```mermaid ` blocks draw.
- A general client test harness, a front-end build step, or a bundler.
- Syntax highlighting or any other added markdown feature.
- Server-side rendering of diagrams (would need a JS engine inside the binary).

## Decisions

### D1: Vendor one pinned UMD bundle, shipped inside the binary

`src/web/vendor/mermaid.min.js` holds a pinned Mermaid release (12.x at the time of writing, exact version and SHA-256 recorded in `src/web/vendor/README.md` next to the upstream MIT notice), embedded with `include_str!` and served from `GET /mermaid.js` as `text/javascript; charset=utf-8` with `Cache-Control: no-cache`, like the other assets. It exposes `globalThis.mermaid`, so a plain `<script src="/mermaid.js">` creates by us works under `script-src 'self'` without module wiring.

The binary grows by roughly the bundle's size. That is the price of offline, same-origin, single-binary delivery; it is paid once in binary size, not in page weight, because (D5) the bundle is never fetched unless a diagram is on screen.

Alternatives considered: a CDN `<script>` (breaks offline, adds a third-party script origin — contradicts the viewer's posture), the ESM build with dynamic `import()` (works, but adds module-script plumbing for no gain in a single-page, no-bundler setup), pre-rendering server-side (needs a JS engine or a build-time CLI; the repo deliberately has neither), and shipping a smaller non-Mermaid renderer (not Mermaid syntax).

### D2: Relax exactly one CSP source, and pin the line in the test

`CSP` becomes:

```
default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'
```

Mermaid writes a `<style>` element into the SVG it produces and sets style attributes on generated nodes; both are governed by `style-src` and both are refused without `'unsafe-inline'`. Nothing else is relaxed: script stays same-origin and inline-script-free (verified: the bundle needs no `eval`), connections stay same-origin, and `default-src 'none'` still blocks images, fonts, frames, and any remote load a diagram might name.

Why this is acceptable here: user content never becomes markup (markdown HTML is escaped, so the only inline style in the document is produced by Mermaid from an escaped source), scripts cannot be injected, and the page is a local read-only viewer with no session or secrets to exfiltrate. CSS-only injection can at worst restyle the viewer.

`every_screen_address_serves_the_shell_with_a_tight_csp` changes from "the exact old string, no `unsafe-inline` anywhere" to: the exact new string; `script-src` is exactly `'self'` with no `unsafe-inline` or `unsafe-eval`; `connect-src` is `'self'`; `default-src` is `'none'`; and the same assertions hold on every screen address. The comment above `CSP` records why the style exception exists, so a later reader does not "clean it up" and break diagrams.

Alternatives considered: keeping `style-src 'self'` and rendering each diagram in a sandboxed iframe with its own policy (opaque-origin frame, `postMessage` sizing, per-frame asset loading — far more machinery for the same visual result), stripping Mermaid's `<style>` and re-injecting its CSS through CSSOM (works only until the next Mermaid release changes its output; brittle), and `style-src-elem` with a nonce (Mermaid does not emit nonces, and the viewer has no per-response nonce infrastructure).

### D3: Draw only fenced `mermaid` blocks, only in the Reader body

After `body.innerHTML = data.html`, `renderReaderData` calls `renderDiagrams(body)`:

1. Collect `pre > code.language-mermaid` elements. Nothing found → return without loading the bundle (D5).
2. For each, keep the `pre` in place and remember its `textContent` (the decoded source — entity escaping is undone by `textContent`, so the diagram sees the original characters).
3. `await ensureMermaid()`, then for each block `await mermaid.render(uniqueId(), source)` and, **only on success**, replace the `pre` with a `<figure class="diagram">` holding the returned `svg`. A rejection leaves the source block untouched and appends one `<p class="diagram-error">` note naming the block as not drawn. Replacement after success is what makes the fallback exact: an undrawable diagram is simply never replaced, and a failed block never affects its neighbours.
4. Record `{ source, figure }` in state so an appearance change can redraw (D4). Reset that list whenever the Reader body is rebuilt.

Because the hook is a class name on the rendered HTML, `markdown.rs` does not change; a unit test pins that a ` ```mermaid ` fenced block renders as `<pre><code class="language-mermaid">` with its source escaped, and that a non-`mermaid` fence keeps its own language class. The `language-mermaid` class is part of the contract between server and client.

Ids are `mmd-<n>` from a page-lifetime counter, never reused across redraws, because Mermaid rejects duplicate ids and creates temporary elements keyed by them.

Alternatives considered: a marker attribute injected server-side (an extra markdown code path and a schema for something the standard fence already encodes), a CSS-only `pre:has(code.language-mermaid)` treatment (a diagram is not CSS), and rendering into a detached container first (Mermaid needs a laid-out parent for measurement; a detached node yields zero-width SVGs).

### D4: Mermaid `base` theme from the viewer's own tokens, redrawn on appearance change

Configuration is one object built at load and rebuilt when the appearance changes:

- `startOnLoad: false` (nothing to auto-scan; the client drives it),
- `securityLevel: "strict"` (HTML in labels sanitised),
- `htmlLabels: false` (labels become SVG `<text>`, so no HTML or `foreignObject` is injected — fewer surfaces, and text is selectable),
- and, because Mermaid 12's strict level still emits the link a `click` directive asks for, the returned SVG is parsed into an inert `<template>` from which anchors are unwrapped, script/embedded content is dropped, and `on*` attributes are removed before the SVG reaches the page. `result.bindFunctions` is never called.
- `theme: "base"` with `themeVariables` mapped from the viewer's computed tokens (`--page`, `--panel`, `--panel-alt`, `--rule`, `--ink`, `--muted`, `--accent`, `--on-accent`, `--code-bg`, `--code-ink`) plus `fontFamily` set to the viewer's mono stack.

Tokens are hex strings in `app.css`, so `getComputedStyle(document.documentElement).getPropertyValue("--…")` yields values Mermaid accepts; a small `diagramTheme()` function reads whichever palette is in effect, which is exactly what the stylesheet already resolved.

Redraw triggers: `applyAppearance` (explicit Light/Dark/System click) and one `matchMedia("(prefers-color-scheme: dark)")` change listener that fires only while no `data-theme` attribute is set. Each trigger re-runs `mermaid.initialize(config)`, then re-renders the remembered sources into their figures with fresh ids. The body, the file bar, the selected version, and the scroll position are untouched — only the SVG inside each figure is replaced. `applyAppearance` gains a call to this hook; it does not otherwise change.

Alternatives considered: the built-in `default`/`dark` themes (fast, but they fight the terminal palette and ignore the page's tokens), a fixed custom palette (duplicates the appearance definitions a third time), and not redrawing until the next navigation (breaks "diagrams follow the appearance" and looks broken while the dialog is open).

### D5: Lazy asset, sequential drawing, exact fallback

`ensureMermaid()` memoises a promise that appends one `<script src="/mermaid.js">` to `<body>` and resolves on `load`, rejects on `error`. It is called only after a rendered file has produced at least one `mermaid` block, so Changes, Files, the not-found screen, and every file without a diagram make no request for it. A rejection is handled exactly like a render rejection: every pending block falls back to its source, and a page already showing diagrams keeps them.

Drawing is sequential (`for … await`), which keeps the DOM work off a burst of parallel renders and makes failure isolation per block. A page with many diagrams is a rare memory file; correctness beats throughput here.

Assets keep `Cache-Control: no-cache`, consistent with the other files, so a rebuilt binary cannot pair new client code with an old bundle. The refetch cost is paid on 127.0.0.1 and only when a diagram is present; adding a hashed, immutable asset route would be new cache machinery for a local viewer.

Alternatives considered: eager preload in `<head>` (adds 5.5 MB to every visit for a feature most files do not use), parallel renders (faster, muddier failure handling; Mermaid's renderer is not documented as parallel-safe), and inlining the bundle into `app.js` (defeats lazy loading entirely).

### D6: Diagram frame scales to the measure, then scrolls

A diagram is wrapped as `<figure class="diagram">` inside the existing `.file-body`, styled as a framed block with `overflow-x: auto`, a `--panel` background, a `--rule` border, and `--radius: 0` — the terminal idiom, not a floating card. The SVG is `display: block`, `height: auto`, `max-width: 100%`, and `min-width: 34rem`:

- narrower than the measure → drawn at its intrinsic size, left-aligned (max-width never upscales);
- wider than the measure → shrinks with the container until it reaches `34rem`, then stops shrinking and scrolls inside the frame, so labels stay legible and the page itself never gains a horizontal scrollbar;
- below `40rem` viewport the shell is already one column; the frame's own scroll is the way to reach the rest.

The figure is `role="img"` with `aria-label="Diagram"` when Mermaid supplies no better description (Mermaid sets `aria-roledescription` on the SVG); the error note is plain text. The source remains in the memory file, so losing a copy button costs nothing during this change.

Alternatives considered: `max-width: 100%` alone (shrinks a wide diagram until text is illegible), full-size with scroll only (forces scrolling for ordinary diagrams that would fit at 80 %), and scaling with CSS `zoom`/`transform` (blurry text, no scrollbar maths).

### D7: Tests, docs, and version

- `markdown.rs`: pins the `language-mermaid` class and source escaping for fenced diagrams; pins that another language's fence is unaffected.
- `mod.rs`: the CSP test is rewritten as D2 describes; a new route test asserts `/mermaid.js` answers 200 with a JavaScript content type, `no-cache`, and the embedded bundle (length sanity, not the whole body); the existing script-escaping test stays.
- No client test harness is added; the manual pass covers both appearances, a valid flowchart, an invalid diagram, two diagrams in one file, a wide diagram at the 40 % width and at mobile width, a diagram in a past version, an appearance flip while a diagram is shown, and a diagram while offline.
- `README.md`: the Reader description gains diagrams; Development notes the vendored bundle, its licence file, the pinned version, and the single style exception in the CSP with its reason. `Cargo.toml` bumps minor at archive (`0.7.0` → `0.8.0`), per `AGENTS.md`.

## Risks / Trade-offs

- **`style-src 'unsafe-inline'` is a real, permanent weakening** → it is the only documented exception, `script-src` stays `'self'` without `unsafe-inline`/`unsafe-eval`, user content still cannot become markup, and both the CSP comment and the README state why. A future nonce-based route for Mermaid styles is possible without touching the specs.
- **Binary grows by ~5.5 MB (roughly 68 %, from ~8 MB to ~13.5 MB)** → accepted for a single-binary, offline viewer; the asset is lazy, so page weight and startup are unchanged for files without diagrams.
- **Mermaid has a history of XSS when configured loosely** (GitLab 2021, `securityLevel: 'loose'`) → strict level, `htmlLabels: false`, the client-side SVG scrub in D3 (Mermaid 12 keeps `click` links under `strict`), a pinned release, and a security requirement with scenarios checked against the real bundle during implementation.
- **A future Mermaid release could need `eval` or script laxity** → the pinned version is deliberate; upgrading is a change of its own that re-runs the CSP tests and the security scenarios. The design does not accept an `unsafe-eval` future silently.
- **Large diagrams block the main thread while rendering** → sequential rendering, no render for files without diagrams, and the bundle only loads on demand; a pathological file degrades to a slow draw, not a broken page.
- **Reading tokens through `getComputedStyle` couples diagrams to token names** → the mapping is one function, the tokens are already the sheet's contract, and a missing token falls back to Mermaid's base theme rather than throwing.
- **`no-cache` on a 5.5 MB asset refetches it on every load of a diagram file** → local-only traffic; correctness after a rebuild matters more. Revisit only if the viewer ever serves a remote host (where `--host` already warns).
- **`min-width: 34rem` can still overflow the frame on very wide diagrams** → that is the intent (scroll, not shrink), and `.diagram` is the scroll container; verify no page-level scrollbar appears at the 40 % and mobile widths.
- **System appearance flips are only noticed while no explicit theme is stored** → the redraw listener is installed on the media query and short-circuits when `data-theme` is set, mirroring how the stylesheet itself resolves appearance.

## Migration Plan

Purely additive: same binary shape, same `web` subcommand, same addresses and JSON, no store or memory-root change. Diagrams appear for files that contain a `mermaid` fence; other files render exactly as before. Rollback is reverting the commit — the vendored file goes with it and the CSP returns to its previous form; no persisted data or migration exists. The version bump lands with the archive.
