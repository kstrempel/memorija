# Design

## Context

See proposal.md — Why. The viewer is three embedded assets served by
`src/web/mod.rs`: `index.html` (the shell and shared header), `app.css`, and
`app.js`, `include_str!`ed with `Cache-Control: no-cache` under a strict CSP
(`default-src 'none'; script-src 'self'; style-src 'self'; …`) that forbids
inline script, inline style, and inline event handlers (change
`2026-11-10-separate-web-ui-views`, design D2). There is no front-end build
step, no framework, and no server-side rendering; screens are rendered client
side after `DOMContentLoaded`-time execution of `app.js` at the end of `<body>`.

`app.css` today declares six colour tokens on `:root` (`--border`, `--muted`,
`--accent`, `--bg`, `--ok`, `--bad`) and then hard-codes light values in several
places: `body` colour `#222` and background `#fff`, `.chip` background `#fff`,
`.new-changes` background `#fff` and its shadow, and `.reader-title` colour.
Those hard-coded values are the main obstacle: a dark palette alone would leave
chips, the floating "new changes" button, and reader titles light.

Constraints that shape this design:

- The CSP must not be loosened; a theme cannot be applied by an inline
  bootstrap script, an inline `<style>`, or an inline `style` attribute.
- The viewer stays read-only: no server endpoint, cookie, or file may record an
  appearance choice.
- Assets remain embedded and hand-written; no new dependency and no build step.
- Every screen is built by the same sheet, so an appearance defined in tokens
  covers Changes, Files, Reader, not-found, and empty states together.

## Goals / Non-Goals

**Goals:**

- One stylesheet that yields a complete light and a complete dark appearance,
  with no light-only value left in a component rule.
- No wrong-appearance flash on load, in either direction (dark OS with no
  choice, or an explicit choice that differs from the OS).
- An explicit choice survives reloads and viewer restarts in the same browser.
- Behaviour comes from CSS where CSS can do it, so system changes apply with no
  JS listener and no re-render.

**Non-Goals:**

- More than two appearances (no high-contrast, sepia, or custom palettes).
- A server-side setting, a CLI flag, or a config file for appearance.
- Re-theming the CLI's terminal output.
- Automated browser testing of the rendered appearance; the repository has no
  browser test harness and this change does not add one.

## Decisions

### D1: Tokens carry the appearance; every rule uses tokens

The appearance is a set of custom properties on `:root`. Tokens gain `--fg`
(body text), `--surface` (page background), `--raised` (chip, button, and
floating-control background), and `--shadow`, so the hard-coded `#fff` and
`#222` disappear from component rules. The existing `--border`, `--muted`,
`--accent`, `--bg`, `--ok`, and `--bad` keep their names and meaning, which
keeps the diff in `app.css` small: the body of the sheet changes only where it
hard-codes a colour.

Alternatives considered:

- **Two stylesheets switched by JS.** Duplicates every component rule and makes
  a wrong-appearance flash unavoidable on the load path; rejected.
- **A single dark stylesheet with `filter: invert()`.** Destroys the accent and
  status colours and inverts markdown images' semantics; rejected.
- **`light-dark()` colour function.** Elegant, but support in the browsers this
  local viewer runs in is not uniform and the fallback would silently drop to
  the light palette; rejected in favour of tokens.

### D2: Dark values live in one block, selected by media query or attribute

```css
:root { /* light tokens, color-scheme: light */ }

@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) { /* dark tokens, color-scheme: dark */ }
}

:root[data-theme="dark"]  { /* the same dark tokens */ }
:root[data-theme="light"] { /* the light tokens again */ }
```

The dark and light blocks are written once and repeated once, adjacent, under a
comment saying the two copies must stay in step. CSS cannot share a declaration
block between a media query and an attribute selector, so a second copy is the
cheapest correct option; the alternative — always setting `data-theme` from JS —
reintroduces the flash this design avoids (D3).

`color-scheme` is set in each block (`light` / `dark`) so form controls,
scrollbars, and the default canvas match the chosen appearance instead of
staying light inside a dark page.

Because `:root:not([data-theme])` is the "no explicit choice" state and the
media query reacts to the OS, a system appearance change applies live with no
listener: the CSS re-evaluates. No `matchMedia` code is needed, and none is
added.

### D3: A tiny same-origin bootstrap script in `<head>`, not inline script

`index.html` gets one new blocking element in `<head>`:
`<script src="/theme.js"></script>`. It runs before `<body>` is parsed, reads the
stored choice, and sets `document.documentElement.dataset.theme` to `"dark"` or
`"light"` — or removes the attribute for "system". Because it is same-origin it
is allowed by `script-src 'self'`, so the CSP is unchanged.

Why not the alternatives:

- **Inline script in `<head>`.** Would need `'unsafe-inline'` (or a hash) in the
  CSP; rejected — the policy is deliberately strict.
- **Apply the stored choice from `app.js` at the end of `<body>`.** CSS would
  already have painted the system appearance, so a user on a light OS who chose
  Dark sees a white flash on every reload — the common case, not an edge case.
- **Only CSS, drop the explicit choice.** Contradicts the chosen behaviour.

`theme.js` is a fourth embedded asset served at `GET /theme.js`, added as one
route beside `/app.css` and `/app.js`, with the same content type and
`no-cache` handling.

### D4: The stored choice is a browser-local key with three states

`localStorage["memorija.theme"]` holds `"light"` or `"dark"`; the key is absent
when no choice was made, which is the "System" state. Choosing System removes
the key rather than writing `"system"`, so there is exactly one representation
of "no choice" — the same state a first visit has.

- All access is wrapped so a throwing or unavailable `localStorage` (private
  mode, storage disabled) degrades to "no choice": `theme.js` still runs, the
  page follows the system preference, and an in-session choice still applies
  until the next load.
- The value is validated on read; anything other than `"light"` or `"dark"` is
  treated as no choice.
- The key is origin-scoped, so `http://127.0.0.1:8081` and a viewer started on
  another port or host remember separately. That is browser behaviour, not a
  bug, and is worth a line in the README.
- Nothing is sent to the server; the viewer's read-only guarantee is untouched.

### D5: The control is static markup in the shared header, wired by `app.js`

`index.html`'s `<header class="site-header">` gains, after the screen nav:

```html
<div class="appearance" role="group" aria-label="Appearance">
  <button type="button" class="chip" data-theme-choice="light" aria-pressed="false">Light</button>
  <button type="button" class="chip" data-theme-choice="dark" aria-pressed="false">Dark</button>
  <button type="button" class="chip" data-theme-choice="system" aria-pressed="false">System</button>
</div>
```

- Buttons, not links: choosing an appearance must not add a history entry or
  touch the address (design D3 of the previous change keeps screen state in the
  address, and appearance is deliberately not part of it).
- The existing `.chip` styling is reused, so the control matches the Changes
  timespan chips; `.chip[aria-pressed="true"]` already styles the active state,
  and the header's flex layout wraps it next to the nav on narrow screens.
- `app.js` sets `aria-pressed` on the three buttons from the effective state and
  attaches one delegated click handler on the group: store the choice (or
  remove the key), set or remove the `data-theme` attribute, re-mark the
  buttons. The screen itself is not re-rendered — a pure CSS change — so the
  control never disturbs the Reader, the feed, or a filter field's cursor.
- Markup is static so the control is served by the shell and testable without a
  browser, and it degrades to inert buttons only when `app.js` has not loaded —
  a page that already renders nothing without JS.

### D6: No endpoint, no state, no schema change

The appearance is entirely client-side: no new API, no change to `/api/*` JSON,
no cookie, no version record, and no memory-root read beyond what the viewer
already does. `src/web/mod.rs` changes only to embed and route `theme.js`; the
CSP string stays byte-identical.

## Risks / Trade-offs

- **Duplicated palette blocks drift** → the two copies sit adjacent under a
  comment; the shell/asset tests assert both `prefers-color-scheme` and
  `[data-theme="dark"]` selectors exist in `app.css`, and any future palette
  change is a single-file, single-block edit.
- **A missed hard-coded colour leaves a light patch** → the implementation sweep
  removes every literal colour from component rules; the review checklist is
  "no `#` colour outside the token blocks", and each screen is checked by eye in
  both appearances (the repository has no automated visual test).
- **Contrast in the dark palette is not measurable here** → target ≥ 4.5:1 for
  body text and links and ≥ 3:1 for muted text and status colours against their
  actual backgrounds; final values are tuned during implementation against those
  targets.
- **An extra asset and one blocking `<head>` script** → `theme.js` is a few
  dozen bytes, same-origin, and cached with `no-cache` like the others; the cost
  is one more embedded file and one more route, not a new dependency.
- **A returning user's appearance becomes OS-driven** → until now the viewer was
  always light; with no stored key a dark-OS user now gets dark. This is the
  intended default and is called out in the migration plan.
- **No automated check of rendered colours** → accepted; tests pin the served
  markup, the tokens, and the storage key, and the visual result is verified
  manually in both appearances on all three screens.

## Migration Plan

Purely additive for users: same binary, same `web` subcommand, same addresses,
same CSP. Existing browsers have no stored key, so the viewer follows the OS
appearance on first load after the upgrade; the header control restores a fixed
light appearance in one click. No persisted data changes, so rollback is
reverting the commit — a stale `memorija.theme` key is simply ignored by the old
build.

## Open Questions

None. Exact palette values are tuned during implementation within the contrast
targets above.
