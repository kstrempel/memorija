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
  reader: null,
  readerBody: null
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
  state.readerBody = null;
  state.reader = null;
  state.readerTitle = null;
  state.readerOffered = null;
  state.readerUpdateBanner = null;
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

  target.appendChild(
    element(
      "p",
      "muted",
      "Recorded changes, newest first. Select a change to read that version."
    )
  );

  var toolbar = element("div", "toolbar");
  SPANS.forEach(function (choice) {
    var chip = link(changesAddress(choice, filter), choice, "chip");
    if (choice === span) {
      chip.className = "chip selected";
      chip.setAttribute("aria-current", "true");
    }
    toolbar.appendChild(chip);
  });

  var input = document.createElement("input");
  input.type = "search";
  input.placeholder = "Filter by path";
  input.value = filter;
  input.setAttribute("aria-label", "Filter changes by path");
  input.addEventListener("input", function () {
    if (!state.changesView) {
      return;
    }
    state.changesView.filter = input.value;
    replaceChangesAddress(state.changesView.span, input.value);
    renderChangesFeed();
  });
  toolbar.appendChild(input);
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

// One feed row: time, path, cause, `+N -M`, and the recorded version. It is
// a real link to the Reader at that version (design D7).
function changeRow(change) {
  var href =
    "/file?path=" +
    encodeURIComponent(change.path) +
    "&version=" +
    change.seq +
    "&from=changes";
  var row = link(href, "", "row");
  row.appendChild(element("span", "when", change.recorded_at.slice(11, 19)));
  row.appendChild(element("span", "file", change.path));
  row.appendChild(element("span", "cause", change.cause));
  var size = element("span", "size");
  size.appendChild(element("span", "added", "+" + change.added));
  size.appendChild(element("span", "removed", " -" + change.removed));
  row.appendChild(size);
  row.appendChild(element("span", "version", "v" + change.seq));
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

  target.appendChild(
    element(
      "p",
      "muted",
      "Every memory file in your memory directory, whether or not it has recorded changes."
    )
  );

  var toolbar = element("div", "toolbar");
  var input = document.createElement("input");
  input.type = "search";
  input.placeholder = "Filter by path";
  input.value = filter;
  input.setAttribute("aria-label", "Filter files by path");
  input.addEventListener("input", function () {
    if (!state.filesView) {
      return;
    }
    state.filesView.filter = input.value;
    history.replaceState(null, "", filesAddress(input.value));
    renderFilesList();
  });
  toolbar.appendChild(input);
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
  var filter = state.filesView.filter;
  var filtered = state.files.filter(function (file) {
    return !filter || file.path.indexOf(filter) !== -1;
  });
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
      element("h2", "day", directory === "" ? "Top level" : directory + "/")
    );
    var rows = element("div", "rows");
    groups.get(directory).forEach(function (file) {
      rows.appendChild(fileRow(file));
    });
    list.appendChild(rows);
  });
}

// One file row: path, line count, latest version, and last change time. It
// opens the Reader at the file's latest version (design D4).
function fileRow(file) {
  var row = link(
    "/file?path=" + encodeURIComponent(file.path) + "&from=files",
    "",
    "row"
  );
  row.appendChild(element("span", "file", file.path));
  var count =
    file.lines === null
      ? "—"
      : file.lines + (file.lines === 1 ? " line" : " lines");
  row.appendChild(element("span", "lines", count));
  if (file.latest_version !== null) {
    row.appendChild(element("span", "version", "v" + file.latest_version));
  }
  if (file.last_changed) {
    row.appendChild(
      element(
        "span",
        "when",
        file.last_changed.slice(0, 10) + " " + file.last_changed.slice(11, 16)
      )
    );
  }
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

  // The back link is a display hint carried in the address (design D9).
  var back = link(
    from === "files" ? "/files" : "/changes",
    from === "files" ? "← All files" : "← Recent changes",
    "back"
  );
  target.appendChild(back);

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

  state.readerTitle = element("h2", "reader-title", path);
  target.appendChild(state.readerTitle);

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
  if (state.readerTitle) {
    state.readerTitle.textContent =
      data.path + (data.version === null ? "" : " · v" + data.version);
  }

  renderVersionStrip(slot, data);

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
  } else if (data.version !== null) {
    slot.appendChild(
      element(
        "p",
        "muted selected-version",
        describedVersion(data)
      )
    );
  }

  var body = element("div", "file-body");
  // The only innerHTML assignment: the server rendered and escaped the
  // markdown, and this is the existing trust boundary (design D7).
  body.innerHTML = data.html;
  slot.appendChild(body);
}

// The scrollable strip of every recorded version, oldest to newest, with the
// selection marked and scrolled into view (design D8).
function renderVersionStrip(slot, data) {
  if (!data.history || data.history.length === 0) {
    return;
  }
  var strip = element("div", "versions");
  strip.setAttribute("aria-label", "Recorded versions");
  var selected = null;
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
    if (entry.seq === data.version) {
      chip.className = "chip selected";
      chip.setAttribute("aria-current", "true");
      selected = chip;
    }
    strip.appendChild(chip);
  });
  slot.appendChild(strip);
  if (selected) {
    strip.scrollLeft =
      selected.offsetLeft - strip.clientWidth / 2 + selected.clientWidth / 2;
  }
}

// The selected record described in words: version, time, cause, and size.
function describedVersion(data) {
  var entry = (data.history || []).filter(function (candidate) {
    return candidate.seq === data.version;
  })[0];
  if (!entry) {
    return "v" + data.version;
  }
  return (
    "v" +
    entry.seq +
    " · " +
    entry.recorded_at +
    " · " +
    entry.cause +
    " · +" +
    entry.added +
    " -" +
    entry.removed
  );
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
  slot.insertBefore(banner, slot.firstChild);
  state.readerUpdateBanner = banner;
}

translateLegacyAddress();
render();
