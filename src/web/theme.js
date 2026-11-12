"use strict";

// The appearance bootstrap (design D3): loaded from <head> so the stored
// choice is on <html> before the body paints. It is a separate same-origin
// file rather than an inline script, so the viewer's Content-Security-Policy
// stays strict.
//
// localStorage["memorija.theme"] holds "light" or "dark"; an absent key - or
// an unreadable store - means "system", which the stylesheet handles with its
// prefers-color-scheme media query (design D2, D4).

(function () {
  var root = document.documentElement;
  var stored = null;
  try {
    stored = window.localStorage.getItem("memorija.theme");
  } catch (error) {
    // Storage unavailable: behave as if no choice was made.
    stored = null;
  }
  if (stored === "light" || stored === "dark") {
    root.setAttribute("data-theme", stored);
  } else {
    root.removeAttribute("data-theme");
  }
})();
