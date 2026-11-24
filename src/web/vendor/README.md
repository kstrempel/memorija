# Vendored Mermaid

`mermaid.min.js` is an unmodified upstream build, embedded in the binary and
served by the viewer at `/mermaid.js` (design D1, D5 of
`openspec/changes/add-mermaid-diagram-rendering`). It is vendored rather than
loaded from a CDN so the viewer stays offline-capable and requests no
third-party origin.

- Package: `mermaid`
- Version: `12.3.0`
- Upstream file: <https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js>
- SHA-256: `336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3`
- License: MIT (below)

## Why this build is safe for the viewer's CSP

The viewer allows script only from its own origin and forbids inline script,
so the bundle must run as a plain same-origin script and must not need
`'unsafe-eval'`. This build satisfies both:

```sh
grep -c 'globalThis\["mermaid"\]' mermaid.min.js   # 1 (exposes the global)
grep -c 'new Function' mermaid.min.js              # 0
grep -c 'eval(' mermaid.min.js                     # 0
```

Re-check these three commands, and the checksum, when upgrading the bundle.
An upgrade that needs `'unsafe-eval'` is not acceptable to this viewer: change
the diagrams' behaviour or the feature instead of widening `script-src`.

## Updating

Download the new upstream file, replace `mermaid.min.js`, update the version,
URL and checksum above, then run `cargo test` (the `/mermaid.js` route test and
the CSP test pin the contract) and walk the diagram scenarios in the web
viewer. `Cache-Control: no-cache` on the asset means a rebuilt binary never
pairs new client code with a stale bundle.

## Upstream license

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
