"use strict";

// The preference bootstrap (design D6): loaded from <head> so the stored
// appearance and page width are on <html> before the body paints. It is a
// separate same-origin file rather than an inline script, so the viewer's
// Content-Security-Policy stays strict.
//
// localStorage["memorija.theme"] holds "light" or "dark"; an absent key - or
// an unreadable store - means "system", which the stylesheet handles with its
// prefers-color-scheme media query (design D3, D4).
//
// localStorage["memorija.width"] holds the shell width as an integer percentage
// of the window, clamped to the range the settings dialog offers. An absent or
// unparsable value leaves the stylesheet's default in place (design D5). The
// property is written through CSSOM, so no inline style attribute - which the
// CSP forbids - is involved.

(function () {
  var root = document.documentElement;

  // A throwing or unavailable store must never break the page: it behaves as
  // if no choice was made (design D4).
  function read(key) {
    try {
      return window.localStorage.getItem(key);
    } catch (error) {
      return null;
    }
  }

  var theme = read("memorija.theme");
  if (theme === "light" || theme === "dark") {
    root.setAttribute("data-theme", theme);
  } else {
    root.removeAttribute("data-theme");
  }

  var stored = read("memorija.width");
  if (stored !== null) {
    var width = parseInt(stored, 10);
    if (!isNaN(width)) {
      width = Math.min(100, Math.max(40, width));
      root.style.setProperty("--shell", String(width));
    }
  }
})();
