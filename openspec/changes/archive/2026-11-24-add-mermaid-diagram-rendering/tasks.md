# Tasks

## 1. Vendor the renderer and serve it

- [x] 1.1 Add a pinned Mermaid build at `src/web/vendor/mermaid.min.js` (record the exact version, upstream URL and SHA-256 in `src/web/vendor/README.md` alongside the upstream MIT license text; confirm the bundle assigns `globalThis["mermaid"]` and contains no `eval(` or `new Function(` — verify with `grep -c "globalThis\[\"mermaid\"\]"` and `grep -c "new Function"` against the vendored file)
- [x] 1.2 Embed the bundle with `include_str!` in `src/web/mod.rs`, add `GET /mermaid.js` to the router and serve it as `text/javascript; charset=utf-8` with `Cache-Control: no-cache`; verify with a new Rust test that the route answers 200, carries that content type and `no-cache`, and returns the embedded bundle (length sanity, not the whole body) — `cargo test --lib web::tests`

## 2. Relax exactly one CSP source

- [x] 2.1 Change the `CSP` constant to `default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'` and extend its doc comment with why the style exception exists (Mermaid writes a `<style>` element and style attributes into its SVG), so a later reader does not remove it; verify the comment names the exception and the spec requirement it serves
- [x] 2.2 Rewrite `every_screen_address_serves_the_shell_with_a_tight_csp` to assert the exact new string, `script-src` exactly `'self'` (no `unsafe-inline`, no `unsafe-eval`), `connect-src 'self'` and `default-src 'none'` on every screen address; verify with `cargo test --lib web::tests::every_screen_address_serves_the_shell_with_a_tight_csp`

## 3. Draw diagrams in the Reader

- [x] 3.1 Pin the client hook in `src/web/markdown.rs` tests: a ` ```mermaid ` fence renders as `<pre><code class="language-mermaid">` with its source escaped, and a fence with another language keeps its own class and text; verify with `cargo test --lib web::markdown`
- [x] 3.2 Implement `renderDiagrams(body)` in `src/web/app.js`: collect `pre > code.language-mermaid` after `body.innerHTML = data.html`, read the source from `textContent`, render with `mermaid.render` under a page-lifetime id counter, replace each `pre` with `<figure class="diagram">` only on success, and append one `.diagram-error` note per failed block while leaving its source in place; verify in `memorija web` that a file with one valid and one invalid block draws the valid one and keeps the invalid block's source plus the note
- [x] 3.3 Load `/mermaid.js` lazily: memoise `ensureMermaid()` as a promise appending one `<script src="/mermaid.js">`, call it only when at least one block was found, and treat a load or render rejection as the same fallback; verify in the browser devtools that a file without a diagram makes no `/mermaid.js` request, and that blocking that request leaves diagram sources readable with the note
- [x] 3.4 Keep the rest of the file intact around diagrams: confirm a file with two diagrams draws both in order, a non-diagram code block is unchanged, and a diagram in a past version draws while the past-version marking stays; verify in `memorija web` on a file with history

## 4. Theme diagrams with the viewer's tokens

- [x] 4.1 Add `diagramTheme()` and the Mermaid configuration (`startOnLoad: false`, `securityLevel: "strict"`, `htmlLabels: false`, `theme: "base"`, `themeVariables` mapped from the computed `--page`, `--panel`, `--panel-alt`, `--rule`, `--ink`, `--muted`, `--accent`, `--on-accent`, `--code-bg`, `--code-ink`, and `fontFamily` from the mono stack); verify a drawn diagram's labels and shapes use the current palette in both appearances
- [x] 4.2 Redraw on appearance change: remember each `{ source, figure }` per Reader body, re-run `mermaid.initialize` and re-render with fresh ids from `applyAppearance` and from one `matchMedia("(prefers-color-scheme: dark)")` change listener that short-circuits while `data-theme` is set; verify that switching Light↔Dark with the settings dialog open redraws the diagram, keeps the shown version and scroll position, and leaves the dialog open, and that a system flip while on System redraws without a reload

## 5. Keep diagrams inside the page

- [x] 5.1 Style `.diagram` as a framed, `overflow-x: auto` block using the existing tokens (`--panel`, `--rule`, `--radius: 0`), with the SVG at `display: block`, `height: auto`, `max-width: 100%`, `min-width: 34rem`, and `.diagram-error` as muted text; verify in both appearances that a small diagram is not stretched and keeps the terminal idiom
- [x] 5.2 Vertically rhythm the figure against prose, code blocks and tables in `.file-body`; verify a file interleaving paragraphs, a diagram, a table and a code block reads evenly in both appearances
- [x] 5.3 Verify a wide diagram at the smallest page width and at mobile width: the page gains no horizontal scrollbar, the diagram scrolls inside its frame, labels stay legible, and nothing overflows the shell — check with the settings slider at 40 % and a narrow browser window

## 6. Documentation and integration

- [x] 6.1 Update `README.md`: the Reader description mentions diagrams; Development documents the vendored bundle (version, license file, where it is served), the single `style-src` exception and why it exists, and that diagrams load only for files that contain one; verify the documented file paths and commands exist as written
- [x] 6.2 Run `cargo test` and confirm the whole suite passes, including the CSP, `/mermaid.js` route and markdown hook tests
- [x] 6.3 Walk the manual pass once end to end on a built binary (`cargo run -- web`): valid flowchart, invalid diagram, two diagrams in one file, diagram in a past version, appearance flip while shown, wide diagram at 40 % and mobile widths, and offline acceptance with the network off; confirm each matches its spec scenario

## Workflow follow-up

- Bump the crate version in `Cargo.toml` (`0.7.0` → `0.8.0`) and update `README.md` at archive, per `AGENTS.md`.
- Archive the change after review.
