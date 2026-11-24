"use strict";

// The client router and the screen renderers live here; the server embeds
// this file and serves it as is, with no build step.
//
// Addresses (design D3):
//   /                              Changes (default), no filters in the URL
//   /changes?span=24h|7d|all&path= Changes with filters
//   /files?q=                      Files with a path filter
//   /file?path=<rel>&version=<n>&from=changes|files
//                                  Reader at the current or a recorded version
//   /?path=<rel>                   legacy file address, translated to the Reader

var screen = document.getElementById("screen");

// The open screen and the data the live stream keeps fresh.
var state = {
  route: { name: "changes", params: new URLSearchParams() },
  changes: [],
  files: [],
  // Per-screen state: the element a screen rebuilds and the values it shows.
  changesView: null,
  changesFeed: null,
  filesView: null,
  filesList: null,
  filesCount: null,
  reader: null,
  readerBody: null,
  readerLines: null,
  // The diagrams drawn in the Reader body, so an appearance change can redraw
  // them from their sources; the token drops draws that a screen change or a
  // new file overtook (design D4).
  diagrams: [],
  diagramsToken: 0
};

// --------------------------------------------------------------- notices --

// A failed fetch or stream leaves the page working and says so inline
// (design D10).
function showNotice(message) {
  var notice = document.getElementById("notice");
  notice.textContent = message;
  notice.hidden = !message;
}

function clearNotice() {
  showNotice("");
}

// --------------------------------------------------------------- helpers --

function element(tag, className, text) {
  var node = document.createElement(tag);
  if (className) {
    node.className = className;
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

// A path filter: the boxed `/` prompt and the search input inside one
// bordered field. The prompt is decorative; the input carries the accessible
// name the screen passes (design D5).
function filterField(value, label, onInput) {
  var field = element("div", "field");
  var prompt = element("span", "prompt", "/");
  prompt.setAttribute("aria-hidden", "true");
  var input = document.createElement("input");
  input.type = "search";
  input.placeholder = "Filter by path";
  input.value = value;
  input.setAttribute("aria-label", label);
  input.addEventListener("input", onInput);
  // Clicking the boxed `/` prompt or the field's padding focuses the input,
  // so the whole visible field behaves like one control.
  field.addEventListener("click", function (event) {
    if (event.target !== input) {
      input.focus();
    }
  });
  field.appendChild(prompt);
  field.appendChild(input);
  return field;
}

// The visible `/` hint beside a list screen's description (spec: path filters
// focus with a slash shortcut).
function filterHint() {
  var hint = element("span", "hint");
  hint.appendChild(document.createTextNode("[press "));
  hint.appendChild(element("kbd", null, "/"));
  hint.appendChild(document.createTextNode(" to filter]"));
  return hint;
}

// --------------------------------------------------------------- settings --

// The settings dialog (design D1, D2): one home for the appearance choice and
// the page width. Both settings apply live, so the dialog only opens and
// closes - there is nothing to commit and nothing to cancel (design D4).
var settingsDialog = document.getElementById("settings");
var settingsOpen = document.getElementById("settings-open");

function closeSettings() {
  if (!settingsDialog) {
    return;
  }
  if (typeof settingsDialog.close === "function") {
    settingsDialog.close();
  } else {
    // Without the platform method there is no `close` event, so focus returns
    // here instead (design D2).
    settingsDialog.removeAttribute("open");
    settingsOpen.focus();
  }
}

if (settingsDialog && settingsOpen) {
  settingsOpen.addEventListener("click", function () {
    if (typeof settingsDialog.showModal === "function") {
      settingsDialog.showModal();
    } else {
      settingsDialog.setAttribute("open", "");
    }
  });

  settingsDialog.addEventListener("click", function (event) {
    if (event.target.closest("[data-settings-close]")) {
      closeSettings();
      return;
    }
    // The backdrop belongs to the dialog, so a click outside the panel arrives
    // here with the dialog as its target. Compare coordinates rather than the
    // target so clicks on the dialog's own padding do not count as outside.
    var bounds = settingsDialog.getBoundingClientRect();
    var outside =
      event.clientX < bounds.left ||
      event.clientX > bounds.right ||
      event.clientY < bounds.top ||
      event.clientY > bounds.bottom;
    if (outside) {
      closeSettings();
    }
  });

  // Escape closes a native dialog and fires the same event, so focus returns to
  // the control that opened it however the dialog was dismissed (design D2).
  settingsDialog.addEventListener("close", function () {
    settingsOpen.focus();
  });
}

// ------------------------------------------------------------- appearance --

// The appearance control (design D3). prefs.js already put the stored
// choice on <html> before the body painted; this keeps the dialog's segments
// marked and records new choices. Choosing an appearance is a pure CSS change,
// so the visible screen - its filters, its scroll position, the Reader's
// version - is never rebuilt. The no-choice state needs no listener: the
// stylesheet follows the system preference through its media query.
var THEME_KEY = "memorija.theme";

// The stored choice, or "system" when none was made (the key is absent) or the
// store is unreadable.
function storedTheme() {
  try {
    var value = window.localStorage.getItem(THEME_KEY);
    return value === "light" || value === "dark" ? value : "system";
  } catch (error) {
    return "system";
  }
}

function markAppearance(choice) {
  var buttons = document.querySelectorAll("[data-theme-choice]");
  for (var i = 0; i < buttons.length; i++) {
    var pressed = buttons[i].dataset.themeChoice === choice;
    buttons[i].setAttribute("aria-pressed", pressed ? "true" : "false");
  }
}

// "system" removes the attribute, leaving the stylesheet's
// prefers-color-scheme media query in charge.
function applyAppearance(choice) {
  if (choice === "light" || choice === "dark") {
    document.documentElement.setAttribute("data-theme", choice);
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
  markAppearance(choice);
  // Diagrams carry colours of their own, so they are redrawn for the new
  // palette; everything else is pure CSS and is left alone (design D4).
  redrawDiagrams();
}

// Persist the choice for later visits. A throwing or unavailable store must not
// break the page (design D4): the appearance still applies until the next load.
function storeAppearance(choice) {
  try {
    if (choice === "light" || choice === "dark") {
      window.localStorage.setItem(THEME_KEY, choice);
    } else {
      window.localStorage.removeItem(THEME_KEY);
    }
  } catch (error) {
    // Storage unavailable: the choice lasts for the current page only.
  }
}

var appearanceControl = document.getElementById("settings");
if (appearanceControl) {
  appearanceControl.addEventListener("click", function (event) {
    var button = event.target.closest("[data-theme-choice]");
    if (!button) {
      return;
    }
    storeAppearance(button.dataset.themeChoice);
    applyAppearance(button.dataset.themeChoice);
  });
  applyAppearance(storedTheme());
}

// ------------------------------------------------------------- page width --

// The page-width control (design D5): the shell reads `--shell` as a percentage
// of the window, so dragging writes that one custom property and nothing else -
// no screen is rebuilt, and filters, scroll position and the open file survive.
// The value is persisted when the drag ends, not on every intermediate value.
var WIDTH_KEY = "memorija.width";
var WIDTH_MIN = 40;
var WIDTH_MAX = 100;
var WIDTH_DEFAULT = 55;

var widthControl = document.getElementById("page-width");
var widthValue = document.getElementById("width-value");

function clampWidth(value) {
  return Math.min(WIDTH_MAX, Math.max(WIDTH_MIN, value));
}

// The width in effect: the stylesheet's `--shell`, which prefs.js already set
// from the stored choice before the body painted. Reading it back keeps the
// control honest about what is on screen.
function currentWidth() {
  var parsed = parseInt(getComputedStyle(document.documentElement).getPropertyValue("--shell"), 10);
  return isNaN(parsed) ? WIDTH_DEFAULT : clampWidth(parsed);
}

function showWidth(width) {
  var text = width + "% of the window";
  if (widthValue) {
    widthValue.textContent = text;
  }
  if (widthControl) {
    widthControl.setAttribute("aria-valuetext", text);
  }
}

function applyWidth(width) {
  document.documentElement.style.setProperty("--shell", String(width));
  showWidth(width);
}

// A throwing or unavailable store must not break the page (design D4): the
// width still applies until the next load.
function storeWidth(width) {
  try {
    window.localStorage.setItem(WIDTH_KEY, String(width));
  } catch (error) {
    // Storage unavailable: the width lasts for the current page only.
  }
}

if (widthControl) {
  var initialWidth = currentWidth();
  widthControl.value = String(initialWidth);
  showWidth(initialWidth);

  widthControl.addEventListener("input", function () {
    applyWidth(clampWidth(parseInt(widthControl.value, 10)));
  });
  widthControl.addEventListener("change", function () {
    storeWidth(clampWidth(parseInt(widthControl.value, 10)));
  });
}

// ---------------------------------------------------------------- diagrams --

// The Reader draws fenced ```mermaid blocks with a vendored Mermaid build
// served from this origin (design D1), fetched only once a rendered file is
// known to contain a diagram (design D5). The source block is never removed
// before its diagram draws, so any failure - an invalid diagram, an
// unreachable renderer - falls back to the source the server already rendered.
var MERMAID_ADDRESS = "/mermaid.js";
var MERMAID_FONT =
  "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
var mermaidLoading = null;
var diagramCounter = 0;

function diagramId() {
  diagramCounter += 1;
  return "memorija-diagram-" + diagramCounter;
}

// One promise for one script element: a second diagram in the same file, or
// the same diagram after a redraw, reuses it. A failed load clears the memo so
// a later navigation can try again.
function ensureMermaid() {
  if (window.mermaid) {
    return Promise.resolve(window.mermaid);
  }
  if (!mermaidLoading) {
    mermaidLoading = new Promise(function (resolve, reject) {
      var script = document.createElement("script");
      script.src = MERMAID_ADDRESS;
      script.async = true;
      script.addEventListener("load", function () {
        if (window.mermaid) {
          resolve(window.mermaid);
        } else {
          reject(new Error("the diagram renderer did not start"));
        }
      });
      script.addEventListener("error", function () {
        mermaidLoading = null;
        reject(new Error("the diagram renderer could not be loaded"));
      });
      document.body.appendChild(script);
    });
  }
  return mermaidLoading;
}

// A token from the appearance in effect, or a neutral fallback for the case
// where the stylesheet could not resolve it. Hex tokens are what Mermaid's
// theme variables expect.
function diagramToken(styles, name, fallback) {
  var value = styles.getPropertyValue(name).trim();
  return value || fallback;
}

// The Mermaid configuration for the palette the stylesheet already resolved:
// `base` plus the viewer's own tokens, so diagrams belong to the same theme as
// the page (design D4). `strict` keeps Mermaid's sanitising and its `click`
// handling off, and SVG text labels keep HTML and foreignObject out of the
// drawn result (spec: diagram content cannot run code).
function diagramConfig() {
  var styles = getComputedStyle(document.documentElement);
  var page = diagramToken(styles, "--page", "#ffffff");
  var panel = diagramToken(styles, "--panel", "#f4f4f4");
  var panelAlt = diagramToken(styles, "--panel-alt", "#e9e9e9");
  var rule = diagramToken(styles, "--rule", "#cccccc");
  var ink = diagramToken(styles, "--ink", "#111111");
  var muted = diagramToken(styles, "--muted", "#555555");
  var accent = diagramToken(styles, "--accent", "#0b62d6");
  var onAccent = diagramToken(styles, "--on-accent", "#ffffff");
  var codeBg = diagramToken(styles, "--code-bg", "#eeeeee");
  var codeInk = diagramToken(styles, "--code-ink", "#8a5a00");
  return {
    startOnLoad: false,
    securityLevel: "strict",
    htmlLabels: false,
    theme: "base",
    fontFamily: MERMAID_FONT,
    themeVariables: {
      fontFamily: MERMAID_FONT,
      background: page,
      primaryColor: panel,
      primaryTextColor: ink,
      primaryBorderColor: rule,
      secondaryColor: panelAlt,
      secondaryTextColor: ink,
      secondaryBorderColor: rule,
      tertiaryColor: codeBg,
      tertiaryTextColor: codeInk,
      tertiaryBorderColor: rule,
      mainBkg: panel,
      nodeBorder: rule,
      nodeTextColor: ink,
      clusterBkg: panelAlt,
      clusterBorder: rule,
      lineColor: muted,
      textColor: ink,
      edgeLabelBackground: page,
      titleColor: ink,
      noteBkgColor: codeBg,
      noteTextColor: codeInk,
      noteBorderColor: rule,
      actorBkg: panel,
      actorBorder: rule,
      actorTextColor: ink,
      actorLineColor: rule,
      signalColor: ink,
      signalTextColor: ink,
      labelBoxBkgColor: panel,
      labelBoxBorderColor: rule,
      labelTextColor: ink,
      loopTextColor: ink,
      activationBkgColor: panelAlt,
      activationBorderColor: rule,
      sequenceNumberColor: onAccent,
      pieTitleTextColor: ink,
      pieSectionTextColor: ink,
      pie1: accent,
      pie2: panelAlt,
      pie3: codeBg,
      pie4: muted,
      pie5: rule,
      pie6: panel,
      pieStrokeColor: page,
      pieOpacity: "1"
    }
  };
}

// Mermaid's SVG is data: a diagram's `click` directives can put links into it,
// and the viewer must not make them live. The string is parsed into an inert
// `<template>`, anchors are unwrapped (keeping their text), and scripts or
// embedded content are dropped before the SVG reaches the document (spec:
// diagram content cannot run code or reach the network). Mermaid's own
// `<style>` element stays - that is the one inline style the CSP allows.
function diagramSvg(markup) {
  var template = document.createElement("template");
  template.innerHTML = markup;
  var svg = template.content.querySelector("svg");
  if (!svg) {
    return null;
  }

  var forbidden = svg.querySelectorAll("script, foreignObject, iframe, object, embed");
  for (var i = 0; i < forbidden.length; i++) {
    forbidden[i].parentNode.removeChild(forbidden[i]);
  }

  var anchors = svg.querySelectorAll("a");
  for (var j = 0; j < anchors.length; j++) {
    var anchor = anchors[j];
    while (anchor.firstChild) {
      anchor.parentNode.insertBefore(anchor.firstChild, anchor);
    }
    anchor.parentNode.removeChild(anchor);
  }

  var nodes = svg.querySelectorAll("*");
  for (var k = 0; k < nodes.length; k++) {
    var attributes = nodes[k].attributes;
    for (var a = attributes.length - 1; a >= 0; a--) {
      if (attributes[a].name.toLowerCase().indexOf("on") === 0) {
        nodes[k].removeAttribute(attributes[a].name);
      }
    }
  }
  return svg;
}

// A temporary node Mermaid creates for a draw is removed again when the draw
// rejects, so a failed diagram leaves nothing behind.
function removeDiagramScratch(id) {
  ["d" + id, id].forEach(function (candidate) {
    var node = document.getElementById(candidate);
    if (node && node.parentNode) {
      node.parentNode.removeChild(node);
    }
  });
}

// Note a block as not drawn. The source block stays where the server put it,
// and the note is added once per block (design D3).
function markDiagramFailed(block) {
  if (!block || !block.parentNode) {
    return;
  }
  if (block.nextSibling && block.nextSibling.className === "diagram-error") {
    return;
  }
  var note = element(
    "p",
    "diagram-error",
    "This diagram could not be drawn; its source is shown above."
  );
  block.parentNode.insertBefore(note, block.nextSibling);
}

// Draw one block into a figure and swap it in only on success. A failure
// leaves the source block exactly as rendered plus the note.
function drawDiagram(mermaid, entry, token) {
  var id = diagramId();
  return mermaid.render(id, entry.source).then(
    function (result) {
      var svg = diagramSvg(result.svg);
      if (state.diagramsToken !== token || !svg) {
        return;
      }
      var figure = element("figure", "diagram");
      figure.appendChild(svg);
      figure.setAttribute("role", "img");
      figure.setAttribute("aria-label", "Diagram");
      entry.block.parentNode.replaceChild(figure, entry.block);
      state.diagrams.push({ source: entry.source, figure: figure });
    },
    function () {
      removeDiagramScratch(id);
      if (state.diagramsToken === token) {
        markDiagramFailed(entry.block);
      }
    }
  );
}

// Draw every mermaid block in a freshly rendered Reader body, one at a time so
// a slow diagram does not starve the others and each failure stays its own.
function renderDiagrams(body) {
  var blocks = body.querySelectorAll("pre > code.language-mermaid");
  if (blocks.length === 0) {
    return;
  }
  var pending = [];
  for (var i = 0; i < blocks.length; i++) {
    pending.push({ source: blocks[i].textContent, block: blocks[i].parentNode });
  }
  var token = state.diagramsToken;
  ensureMermaid().then(
    function (mermaid) {
      mermaid.initialize(diagramConfig());
      var index = 0;
      function next() {
        if (index >= pending.length || state.diagramsToken !== token) {
          return null;
        }
        var entry = pending[index];
        index += 1;
        return drawDiagram(mermaid, entry, token).then(next);
      }
      next();
    },
    function () {
      // No renderer: every block keeps its source and says so.
      pending.forEach(function (entry) {
        if (state.diagramsToken === token) {
          markDiagramFailed(entry.block);
        }
      });
    }
  );
}

// Redraw the diagrams already on screen for the appearance now in effect. Only
// the SVG inside each figure is replaced, so the file, version, scroll and the
// open settings dialog stay as they were (design D4). A redraw that fails
// keeps the previous drawing rather than trading a diagram for a note.
function redrawDiagrams() {
  var mermaid = window.mermaid;
  var entries = state.diagrams;
  if (!mermaid || entries.length === 0) {
    return;
  }
  mermaid.initialize(diagramConfig());
  var token = state.diagramsToken;
  var index = 0;
  function next() {
    if (index >= entries.length || state.diagramsToken !== token) {
      return null;
    }
    var entry = entries[index];
    index += 1;
    var id = diagramId();
    return mermaid.render(id, entry.source).then(
      function (result) {
        var svg = diagramSvg(result.svg);
        if (state.diagramsToken === token && svg && entry.figure.parentNode) {
          entry.figure.textContent = "";
          entry.figure.appendChild(svg);
        }
      },
      function () {
        removeDiagramScratch(id);
      }
    ).then(next);
  }
  next();
}

// With no explicit choice the stylesheet follows the system preference, so a
// flip there changes the palette too; the media query report redraws the
// diagrams that are on screen. An explicit choice wins and needs no listener.
var systemAppearance = window.matchMedia
  ? window.matchMedia("(prefers-color-scheme: dark)")
  : null;
if (systemAppearance && systemAppearance.addEventListener) {
  systemAppearance.addEventListener("change", function () {
    if (!document.documentElement.hasAttribute("data-theme")) {
      redrawDiagrams();
    }
  });
}

// ---------------------------------------------------------------- routing --

// Translate an old `/?path=<file>` bookmark before the first render, without
// adding a history entry (design D3).
function translateLegacyAddress() {
  if (location.pathname !== "/") {
    return;
  }
  var params = new URLSearchParams(location.search);
  var path = params.get("path");
  if (path) {
    history.replaceState(null, "", "/file?path=" + encodeURIComponent(path));
  }
}

// The screen an address names; anything else is the not-found screen.
function parseRoute() {
  var path = location.pathname;
  var params = new URLSearchParams(location.search);
  if (path === "/" || path === "/changes") {
    return { name: "changes", params: params };
  }
  if (path === "/files") {
    return { name: "files", params: params };
  }
  if (path === "/file") {
    return { name: "file", params: params };
  }
  return { name: "notfound", params: params };
}

// Mark the header's current screen; the Reader marks itself.
function markNav(name) {
  var links = document.querySelectorAll(".screens a");
  for (var i = 0; i < links.length; i++) {
    if (links[i].dataset.screen === name) {
      links[i].setAttribute("aria-current", "page");
    } else {
      links[i].removeAttribute("aria-current");
    }
  }
}

// Navigate to an address: one history entry, then render the screen.
function navigate(address, replace) {
  if (replace) {
    history.replaceState(null, "", address);
  } else {
    history.pushState(null, "", address);
  }
  render();
}

// Render the screen the address names.
function render() {
  state.route = parseRoute();
  markNav(state.route.name);
  // Drop the previous screen's rebuild hooks before the new screen claims them.
  state.changesFeed = null;
  state.filesList = null;
  state.filesView = null;
  state.filesCount = null;
  state.readerBody = null;
  state.reader = null;
  state.readerLines = null;
  state.readerOffered = null;
  state.readerUpdateBanner = null;
  // Drop the previous screen's diagrams: a draw still in flight must not land
  // on the new screen (design D4).
  state.diagrams = [];
  state.diagramsToken += 1;
  hideNewChanges();
  screen.textContent = "";
  switch (state.route.name) {
    case "changes":
      renderChanges(screen, state.route.params);
      break;
    case "files":
      renderFiles(screen, state.route.params);
      break;
    case "file":
      renderReader(screen, state.route.params);
      break;
    default:
      renderNotFound(screen);
  }
}

// A same-origin link; every screen navigation uses real <a href> elements so
// middle-click, copy-link, and open-in-new-tab work (design D7).
function link(address, text, className) {
  var anchor = document.createElement("a");
  anchor.href = address;
  anchor.textContent = text;
  if (className) {
    anchor.className = className;
  }
  return anchor;
}

// Intercept plain left clicks on same-origin links and route them in-page.
document.addEventListener("click", function (event) {
  if (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  ) {
    return;
  }
  var anchor = event.target.closest("a");
  if (!anchor || anchor.target) {
    return;
  }
  var href = anchor.getAttribute("href");
  if (!href || href.charAt(0) !== "/") {
    return;
  }
  event.preventDefault();
  navigate(href, false);
});

window.addEventListener("popstate", render);

// The current list screen's filter input, or null on a screen without one.
function currentFilter() {
  if (state.route.name !== "changes" && state.route.name !== "files") {
    return null;
  }
  return screen.querySelector(".field input");
}

// `/` focuses the list filter and Escape releases it (spec: path filters focus
// with a slash shortcut). Modifiers and anything editable are left alone, so
// typing is never hijacked.
document.addEventListener("keydown", function (event) {
  if (event.key === "Escape") {
    var focused = document.activeElement;
    if (focused && focused.matches && focused.matches(".field input")) {
      focused.blur();
    }
    return;
  }
  if (
    event.defaultPrevented ||
    event.metaKey ||
    event.ctrlKey ||
    event.altKey ||
    event.key !== "/"
  ) {
    return;
  }
  var target = event.target;
  if (
    target &&
    target.closest &&
    target.closest("input, textarea, select, [contenteditable]")
  ) {
    return;
  }
  var input = currentFilter();
  if (!input) {
    return;
  }
  event.preventDefault();
  input.focus();
  var end = input.value.length;
  if (input.setSelectionRange) {
    input.setSelectionRange(end, end);
  }
});

// ---------------------------------------------------------------- screens --

// The Changes screen's timespan choices, matching the CLI's common presets
// (design D4); the default is the last 24 hours.
var SPANS = ["24h", "7d", "all"];

function normalizeSpan(span) {
  return SPANS.indexOf(span) === -1 ? "24h" : span;
}

// The Changes address for a filter choice (design D3): the span is always
// recorded, the path filter only when it has text.
function changesAddress(span, filter) {
  var params = new URLSearchParams();
  params.set("span", span);
  if (filter) {
    params.set("path", filter);
  }
  return "/changes?" + params.toString();
}

// Update the address without re-rendering: typing in the filter must not
// rebuild the toolbar and lose the cursor.
function replaceChangesAddress(span, filter) {
  history.replaceState(null, "", changesAddress(span, filter));
}

// A stored `YYYY-MM-DDTHH:MM:SSZ` timestamp `ms` milliseconds before now:
// stored timestamps sort chronologically, so filtering is a string compare.
function isoSeconds(ms) {
  return new Date(ms).toISOString().slice(0, 19) + "Z";
}

// The oldest timestamp the chosen span includes; `""` for all changes.
function spanCutoff(span) {
  var day = 24 * 60 * 60 * 1000;
  if (span === "24h") {
    return isoSeconds(Date.now() - day);
  }
  if (span === "7d") {
    return isoSeconds(Date.now() - 7 * day);
  }
  return "";
}

function filterChanges(changes) {
  var view = state.changesView;
  var cutoff = spanCutoff(view.span);
  return changes.filter(function (change) {
    if (cutoff && change.recorded_at < cutoff) {
      return false;
    }
    return !view.filter || change.path.indexOf(view.filter) !== -1;
  });
}

// Fetch the feed once per screen visit; the live stream updates it later
// (design D5).
function loadChanges() {
  return fetch("/api/changes")
    .then(function (response) {
      return response.json();
    })
    .then(function (data) {
      if (data.error) {
        showNotice(data.error);
        return;
      }
      clearNotice();
      state.changes = data.changes || [];
    })
    .catch(function (error) {
      showNotice("Cannot reach the viewer: " + error);
    });
}

function renderChanges(target, params) {
  var span = normalizeSpan(params.get("span"));
  var filter = params.get("path") || "";
  state.changesView = { span: span, filter: filter };

  var head = element("div", "screen-head");
  head.appendChild(
    element(
      "p",
      "muted",
      "Recorded changes, newest first. Select a change to read that version."
    )
  );
  head.appendChild(filterHint());
  target.appendChild(head);

  var toolbar = element("div", "toolbar");
  SPANS.forEach(function (choice) {
    var chip = link(changesAddress(choice, filter), choice, "chip");
    if (choice === span) {
      chip.className = "chip selected";
      chip.setAttribute("aria-current", "true");
    }
    toolbar.appendChild(chip);
  });

  toolbar.appendChild(
    filterField(filter, "Filter changes by path", function (event) {
      if (!state.changesView) {
        return;
      }
      var input = event.target;
      state.changesView.filter = input.value;
      replaceChangesAddress(state.changesView.span, input.value);
      renderChangesFeed();
    })
  );
  target.appendChild(toolbar);

  var feed = element("div", "feed");
  state.changesFeed = feed;
  target.appendChild(feed);

  loadChanges().then(renderChangesFeed);
}

function renderChangesFeed() {
  var feed = state.changesFeed;
  if (!feed) {
    return;
  }
  feed.textContent = "";
  if (state.changes.length === 0) {
    feed.appendChild(
      element(
        "p",
        "muted empty",
        "No changes recorded yet. As agent sessions create and edit memory files, every change shows up here automatically."
      )
    );
    return;
  }
  var filtered = filterChanges(state.changes);
  if (filtered.length === 0) {
    feed.appendChild(
      element("p", "muted empty", "No changes match the current filters.")
    );
    return;
  }
  var day = "";
  var rows = null;
  filtered.forEach(function (change) {
    var changeDay = change.recorded_at.slice(0, 10);
    if (changeDay !== day) {
      day = changeDay;
      feed.appendChild(element("h2", "day", changeDay));
      rows = element("div", "rows");
      feed.appendChild(rows);
    }
    rows.appendChild(changeRow(change));
  });
}

// One feed row: time, path, and the right cluster of cause, `+N -M`, and the
// recorded version. It is a real link to the Reader at that version; the `▸`
// marker is decorative CSS, not a disclosure control.
function changeRow(change) {
  var href =
    "/file?path=" +
    encodeURIComponent(change.path) +
    "&version=" +
    change.seq +
    "&from=changes";
  var row = link(href, "", "row change-row");
  row.appendChild(element("span", "when", change.recorded_at.slice(11, 19)));
  row.appendChild(element("span", "file", change.path));
  var meta = element("span", "meta");
  meta.appendChild(element("span", "cause", change.cause));
  var size = element("span", "size");
  size.appendChild(element("span", "added", "+" + change.added));
  size.appendChild(element("span", "removed", "-" + change.removed));
  meta.appendChild(size);
  meta.appendChild(element("span", "version", "v" + change.seq));
  row.appendChild(meta);
  return row;
}

// The "N new changes" affordance while the feed is scrolled away from the
// top (design D5): the rows stay put until the user asks for the new ones.
var newChanges = null;

function showNewChanges(count) {
  if (!newChanges) {
    newChanges = element("button", "new-changes");
    newChanges.type = "button";
    newChanges.addEventListener("click", function () {
      hideNewChanges();
      window.scrollTo(0, 0);
      renderChangesFeed();
    });
    document.body.appendChild(newChanges);
  }
  newChanges.textContent =
    count + (count === 1 ? " new change" : " new changes");
  newChanges.hidden = false;
}

function hideNewChanges() {
  if (newChanges) {
    newChanges.hidden = true;
  }
}

function renderFiles(target, params) {
  var filter = params.get("q") || "";
  state.filesView = { filter: filter };

  var head = element("div", "screen-head");
  head.appendChild(
    element(
      "p",
      "muted",
      "Every memory file in your memory directory, whether or not it has recorded changes."
    )
  );
  var meta = element("span", "screen-meta");
  state.filesCount = element("span", "count", "");
  meta.appendChild(state.filesCount);
  head.appendChild(meta);
  target.appendChild(head);

  var toolbar = element("div", "toolbar");
  toolbar.appendChild(
    filterField(filter, "Filter files by path", function (event) {
      if (!state.filesView) {
        return;
      }
      state.filesView.filter = event.target.value;
      history.replaceState(null, "", filesAddress(event.target.value));
      renderFilesList();
    })
  );
  target.appendChild(toolbar);

  var list = element("div", "files");
  state.filesList = list;
  target.appendChild(list);

  loadFiles().then(renderFilesList);
}

// The Files address for a filter choice (design D3).
function filesAddress(filter) {
  return filter ? "/files?q=" + encodeURIComponent(filter) : "/files";
}

function loadFiles() {
  return fetch("/api/files")
    .then(function (response) {
      return response.json();
    })
    .then(function (data) {
      if (data.error) {
        showNotice(data.error);
        return;
      }
      clearNotice();
      state.files = data.files || [];
    })
    .catch(function (error) {
      showNotice("Cannot reach the viewer: " + error);
    });
}

// Same-directory key; the root is the empty key (design D3).
function directoryOf(path) {
  var slash = path.lastIndexOf("/");
  return slash === -1 ? "" : path.slice(0, slash);
}

function renderFilesList() {
  var list = state.filesList;
  if (!list) {
    return;
  }
  list.textContent = "";

  var filter = state.filesView.filter;
  var filtered = state.files.filter(function (file) {
    return !filter || file.path.indexOf(filter) !== -1;
  });
  updateFilesCount(filtered.length, state.files.length, filter);

  if (state.files.length === 0) {
    list.appendChild(
      element(
        "p",
        "muted empty",
        "No memory files yet. Everything in your memory directory appears here, whether or not it has recorded changes."
      )
    );
    return;
  }
  if (filtered.length === 0) {
    list.appendChild(element("p", "muted empty", "No files match the filter."));
    return;
  }

  // Group by directory, directories in path order with the root first.
  var groups = new Map();
  filtered.forEach(function (file) {
    var directory = directoryOf(file.path);
    if (!groups.has(directory)) {
      groups.set(directory, []);
    }
    groups.get(directory).push(file);
  });
  var directories = Array.from(groups.keys()).sort();
  directories.forEach(function (directory) {
    list.appendChild(
      element("h2", "group", directory === "" ? "Top level" : directory + "/")
    );
    var rows = element("div", "rows");
    groups.get(directory).forEach(function (file) {
      rows.appendChild(fileRow(file));
    });
    list.appendChild(rows);
  });
}

// The count names the files currently listed; a filter makes the total visible
// so it is clear what was narrowed away (design D4).
function updateFilesCount(shown, total, filter) {
  if (!state.filesCount) {
    return;
  }
  var noun = total === 1 ? " file" : " files";
  state.filesCount.textContent = filter
    ? shown + " of " + total + noun
    : total + noun;
}

// One file row: path on the left, line count / version / last change as the
// right cluster. It opens the Reader at the file's latest version; the `▸`
// marker is decorative CSS, not a disclosure control.
function fileRow(file) {
  var row = link(
    "/file?path=" + encodeURIComponent(file.path) + "&from=files",
    "",
    "row file-row"
  );
  row.appendChild(element("span", "file", file.path));
  var meta = element("span", "meta");
  var count =
    file.lines === null
      ? "—"
      : file.lines + (file.lines === 1 ? " line" : " lines");
  meta.appendChild(element("span", "lines", count));
  if (file.latest_version !== null) {
    meta.appendChild(
      element("span", "version", "v" + file.latest_version)
    );
  }
  if (file.last_changed) {
    meta.appendChild(
      element(
        "span",
        "stamp",
        file.last_changed.slice(0, 10) +
          " " +
          file.last_changed.slice(11, 16)
      )
    );
  }
  row.appendChild(meta);
  return row;
}

// The Reader's address for a file, an optional version, and its origin
// screen (design D3, D9).
function readerAddress(path, version, from) {
  var params = new URLSearchParams();
  params.set("path", path);
  if (version !== null && version !== undefined && version !== "") {
    params.set("version", version);
  }
  params.set("from", from);
  return "/file?" + params.toString();
}

function loadFile(path, version) {
  var address = "/api/file?path=" + encodeURIComponent(path);
  if (version) {
    address += "&version=" + encodeURIComponent(version);
  }
  return fetch(address)
    .then(function (response) {
      return response.json().then(function (data) {
        return { status: response.status, data: data };
      });
    })
    .catch(function (error) {
      return {
        status: 0,
        data: { error: "Cannot reach the viewer: " + error }
      };
    });
}

function renderReader(target, params) {
  var path = params.get("path");
  var version = params.get("version");
  var from = params.get("from") === "files" ? "files" : "changes";
  state.reader = { path: path, version: version, from: from, data: null };

  // The back link and the shown version's line count share the top row.
  var head = element("div", "screen-head reader-head");
  head.appendChild(
    link(
      from === "files" ? "/files" : "/changes",
      from === "files" ? "← All files" : "← Recent changes",
      "back"
    )
  );
  state.readerLines = element("span", "lines", "");
  head.appendChild(state.readerLines);
  target.appendChild(head);

  if (!path) {
    target.appendChild(
      element(
        "p",
        "muted empty",
        "No file selected. Pick one from Changes or Files."
      )
    );
    return;
  }

  var slot = element("div", "reader");
  state.readerBody = slot;
  target.appendChild(slot);

  loadFile(path, version).then(function (result) {
    if (state.readerBody === slot) {
      renderReaderData(result);
    }
  });
}

function renderReaderData(result) {
  var reader = state.reader;
  var slot = state.readerBody;
  var data = result.data;
  if (!reader || !slot) {
    return;
  }
  slot.textContent = "";
  if (result.status !== 200 || data.error) {
    if (state.readerLines) {
      state.readerLines.textContent = "";
    }
    slot.appendChild(
      element(
        "p",
        "error empty",
        data.error || "That file could not be read."
      )
    );
    return;
  }
  reader.data = data;
  if (state.readerLines && typeof data.lines === "number") {
    state.readerLines.textContent =
      data.lines + (data.lines === 1 ? " line" : " lines");
  }

  renderFileBar(slot, data);

  if (!data.is_current) {
    var past = element("div", "past-version");
    past.appendChild(
      element(
        "span",
        null,
        "Viewing version " +
          data.version +
          " of v" +
          data.latest_version +
          ", not the latest."
      )
    );
    past.appendChild(
      link(
        readerAddress(data.path, null, reader.from),
        "View latest",
        "latest-link"
      )
    );
    slot.appendChild(past);
  }

  var body = element("div", "file-body");
  // The server rendered and escaped this markdown; this assignment and the
  // inert `<template>` parse in `diagramSvg` are the only innerHTML uses, and
  // the existing trust boundary (design D3).
  body.innerHTML = data.html;
  slot.appendChild(body);
  state.diagrams = [];
  state.diagramsToken += 1;
  renderDiagrams(body);
}

// The file bar: identity on the left, the version tabs on the right. HEAD is
// the current content (no version in the address); every recorded version is
// a real link, and the shown tab is filled and scrolled into view (design D6).
function renderFileBar(slot, data) {
  var bar = element("div", "file-bar");
  var name = element("span", "name");
  name.appendChild(element("span", "sig", "::"));
  name.appendChild(element("span", "path", data.path));
  bar.appendChild(name);

  var versions = element("div", "versions");
  versions.setAttribute("aria-label", "Recorded versions");
  var selected = null;

  if (data.history && data.history.length > 0) {
    data.history.forEach(function (entry) {
      var chip = link(
        readerAddress(data.path, entry.seq, state.reader.from),
        "v" + entry.seq,
        "chip"
      );
      chip.title =
        entry.recorded_at +
        " · " +
        entry.cause +
        " (+" +
        entry.added +
        " -" +
        entry.removed +
        ")";
      if (!data.is_current && entry.seq === data.version) {
        chip.className = "chip selected";
        chip.setAttribute("aria-current", "true");
        selected = chip;
      }
      versions.appendChild(chip);
    });
    var head = link(
      readerAddress(data.path, null, state.reader.from),
      "HEAD",
      "chip"
    );
    head.title = "Current content";
    if (data.is_current) {
      head.className = "chip selected";
      head.setAttribute("aria-current", "true");
      selected = head;
    }
    versions.appendChild(head);
  } else {
    var badge = element("span", "chip head-badge", "HEAD");
    versions.appendChild(badge);
  }

  bar.appendChild(versions);
  slot.appendChild(bar);

  if (selected) {
    versions.scrollLeft =
      selected.offsetLeft - versions.clientWidth / 2 + selected.clientWidth / 2;
  }
}

function renderNotFound(target) {
  var heading = document.createElement("h2");
  heading.textContent = "Not found";
  target.appendChild(heading);
  var body = document.createElement("p");
  body.appendChild(document.createTextNode("That address is not part of the viewer. "));
  body.appendChild(link("/", "Go to Changes"));
  target.appendChild(body);
}

// ----------------------------------------------------------- live updates --

// One stream for the page's lifetime (design D5): switching screens never
// reconnects it. The server pushes the whole feed and file index whenever
// the version store or the memory root's shape moves.
var events = new EventSource("/api/events");

events.addEventListener("changes", function (message) {
  var payload;
  try {
    payload = JSON.parse(message.data);
  } catch (error) {
    return;
  }
  if (payload.error) {
    showNotice(payload.error);
    return;
  }
  clearNotice();
  applyPush(payload.changes || [], payload.files || []);
});

// EventSource reconnects on its own; say so while it is down (design D10).
events.onerror = function () {
  showNotice("Live updates paused; reconnecting…");
};

// Apply a pushed payload to whichever screen is visible, without reloading.
function applyPush(changes, files) {
  var arriving = newChangesSince(state.changes, changes);
  state.changes = changes;
  state.files = files;

  if (state.route.name === "changes" && state.changesFeed) {
    if (arriving > 0 && window.scrollY > 40) {
      // Keep the rows the user is reading and offer the new ones instead.
      showNewChanges(arriving);
    } else {
      hideNewChanges();
      renderChangesFeed();
    }
  }

  if (state.route.name === "files" && state.filesList) {
    renderFilesList();
  }

  if (state.route.name === "file" && state.reader && state.reader.data) {
    var entry = files.filter(function (file) {
      return file.path === state.reader.data.path;
    })[0];
    if (entry && entry.latest_version !== null) {
      var shown = state.reader.data.latest_version;
      if (shown === null || entry.latest_version > shown) {
        offerNewerVersion(entry);
      }
    }
  }
}

// How many changes in `next` are new against `previous`: a recorded change
// is unique by path and per-path sequence number.
function newChangesSince(previous, next) {
  var seen = new Set();
  previous.forEach(function (change) {
    seen.add(change.path + ":" + change.seq);
  });
  return next.filter(function (change) {
    return !seen.has(change.path + ":" + change.seq);
  }).length;
}

// The Reader keeps its open content and offers the newer version instead of
// replacing what the user is reading (design D5).
function offerNewerVersion(entry) {
  var slot = state.readerBody;
  if (!slot || state.readerOffered === entry.latest_version) {
    return;
  }
  state.readerOffered = entry.latest_version;
  if (state.readerUpdateBanner && state.readerUpdateBanner.parentNode === slot) {
    slot.removeChild(state.readerUpdateBanner);
  }
  var banner = element("div", "past-version updated");
  banner.appendChild(element("span", null, "This file was updated."));
  banner.appendChild(
    link(
      readerAddress(state.reader.data.path, entry.latest_version, state.reader.from),
      "View v" + entry.latest_version,
      "latest-link"
    )
  );
  var body = slot.querySelector(".file-body");
  if (body) {
    slot.insertBefore(banner, body);
  } else {
    slot.appendChild(banner);
  }
  state.readerUpdateBanner = banner;
  markShownVersion(slot, state.reader.data);
}

// Once a live update makes HEAD stale, the bar marks the version actually on
// screen instead of HEAD, while the banner offers the newer one (design D6).
function markShownVersion(slot, data) {
  if (!slot || !data || data.version === null || data.version === undefined) {
    return;
  }
  var versions = slot.querySelector(".versions");
  if (!versions) {
    return;
  }
  var head = null;
  var target = null;
  var chips = versions.querySelectorAll(".chip");
  for (var i = 0; i < chips.length; i++) {
    if (chips[i].textContent === "HEAD") {
      head = chips[i];
    }
    if (chips[i].textContent === "v" + data.version) {
      target = chips[i];
    }
  }
  if (!head || !target || !head.classList.contains("selected")) {
    return;
  }
  head.classList.remove("selected");
  head.removeAttribute("aria-current");
  target.classList.add("selected");
  target.setAttribute("aria-current", "true");
}

translateLegacyAddress();
render();
