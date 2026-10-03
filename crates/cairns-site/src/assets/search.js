// Filtering and ordering for the index.
//
// The list is in the HTML already, newest first, so the page reads correctly
// with this file blocked or still loading. Everything here is enhancement.
(function () {
  var list = document.getElementById("entries");
  if (!list) return;

  // Only the rows themselves: a row on the open questions page holds a list of
  // its own, and those items are not rows.
  var rows = Array.prototype.slice.call(list.querySelectorAll("li[data-n]")); // newest first
  // The open questions page searches what it shows, not the entries' text.
  var local = list.hasAttribute("data-local");
  // The entries come grouped under a heading per day. A day with nothing left
  // in it after filtering goes too, or the page is a column of empty dates.
  var days = Array.prototype.slice.call(list.querySelectorAll(".day"));
  var box = document.getElementById("q");
  var chips = Array.prototype.slice.call(document.querySelectorAll(".chip[data-area]"));
  var sorters = Array.prototype.slice.call(document.querySelectorAll("button[data-sort]"));
  var status = document.getElementById("status");

  var picked = new Set();
  var order = "new";
  var text = null; // number -> haystack, fetched on demand

  function fromPage() {
    text = new Map();
    rows.forEach(function (row) {
      text.set(row.dataset.n, (row.textContent || "").toLowerCase());
    });
    return text;
  }

  function load() {
    if (text) return Promise.resolve(text);
    if (local) return Promise.resolve(fromPage());
    return fetch("search.json")
      .then(function (r) { return r.json(); })
      .then(function (data) {
        text = new Map();
        data.forEach(function (row) { text.set(String(row.n), row.t.toLowerCase()); });
        return text;
      })
      // Offline, or opened as a file:// page. Titles and summaries still filter.
      .catch(fromPage);
  }

  // Each order is the other reversed, at both levels: the days, and the
  // entries within each day. So a change of order is one reversal of both.
  function reorder() {
    if (!days.length) {
      rows.slice().reverse().forEach(function (row) { list.appendChild(row); });
      rows.reverse();
      return;
    }
    days.forEach(function (day) {
      var inner = day.querySelector("ul");
      var items = Array.prototype.slice.call(inner.children).reverse();
      items.forEach(function (item) { inner.appendChild(item); });
    });
    days.reverse();
    days.forEach(function (day) { list.appendChild(day); });
  }

  function apply() {
    var query = ((box && box.value) || "").trim().toLowerCase();
    var shown = 0;

    rows.forEach(function (row) {
      var areas = (row.dataset.areas || "").split(" ");
      var byArea = picked.size === 0 || areas.some(function (a) { return picked.has(a); });
      var hay = text ? text.get(row.dataset.n) || "" : (row.textContent || "").toLowerCase();
      var inState = !row.dataset.state || row.dataset.state === state;
      var show = inState && byArea && (query === "" || hay.indexOf(query) !== -1);
      row.hidden = !show;
      if (show) shown++;
      // A match may be in the folded part of a long list; a search opens it.
      if (query !== "" && show) {
        Array.prototype.slice.call(row.querySelectorAll("details.more")).forEach(function (d) { d.open = true; });
      }
    });
    days.forEach(function (day) {
      day.hidden = !day.querySelector("li:not([hidden])");
    });

    if (status) {
      var total = rows.filter(function (row) { return !row.dataset.state || row.dataset.state === state; }).length;
      status.textContent =
        picked.size > 0 || query !== "" ? shown + " of " + total + " entries" : "";
    }
  }

  function syncHash() {
    var parts = [];
    if (order === "old") parts.push("sort=old");
    if (state === "closed") parts.push("state=closed");
    if (picked.size) parts.push("area=" + Array.from(picked).join(","));
    var hash = parts.length ? "#" + parts.join("&") : "";
    history.replaceState(null, "", location.pathname + location.search + hash);
  }

  sorters.forEach(function (button) {
    button.addEventListener("click", function () {
      if (order === button.dataset.sort) return;
      order = button.dataset.sort;
      sorters.forEach(function (other) {
        other.setAttribute("aria-pressed", other === button ? "true" : "false");
      });
      reorder();
      syncHash();
    });
  });

  chips.forEach(function (chip) {
    chip.addEventListener("click", function () {
      var area = chip.dataset.area;
      if (picked.has(area)) picked.delete(area);
      else picked.add(area);
      chip.setAttribute("aria-pressed", picked.has(area) ? "true" : "false");
      syncHash();
      apply();
    });
  });

  if (box) {
    box.addEventListener("focus", function () { load().then(apply); }, { once: true });
    box.addEventListener("input", function () { load().then(apply); });
  }

  // A shared link carries its order and its filters - and so does an area
  // pill on an entry page, which links back here with one already chosen.
  function fromHash() {
    var hash = location.hash.replace(/^#/, "");
    // A hash with no `key=value` in it - `#d2026-10-02` from the activity
    // strip, `#closed` on the open questions page - is a place to scroll to,
    // not a change of filters.
    if (hash && hash.indexOf("=") === -1) return;
    picked.clear();
    var wanted = "new";
    var wantedState = "open";
    hash.split("&").forEach(function (part) {
      var pair = part.split("=");
      if (pair[0] === "sort" && pair[1] === "old") wanted = "old";
      if (pair[0] === "state" && pair[1] === "closed") wantedState = "closed";
      if (pair[0] === "area" && pair[1]) {
        decodeURIComponent(pair[1]).split(",").forEach(function (area) { picked.add(area); });
      }
    });
    if (wanted !== order) {
      order = wanted;
      reorder();
    }
    sorters.forEach(function (button) {
      button.setAttribute("aria-pressed", button.dataset.sort === order ? "true" : "false");
    });
    if (states.length) showState(wantedState);
    chips.forEach(function (chip) {
      chip.setAttribute("aria-pressed", picked.has(chip.dataset.area) ? "true" : "false");
    });
    apply();
  }

  // Density is a reader's preference rather than a view of the log, so it is
  // remembered in this browser instead of carried in the link.
  var densities = Array.prototype.slice.call(document.querySelectorAll("button[data-density]"));
  function density(which) {
    list.classList.toggle("compact", which === "compact");
    densities.forEach(function (button) {
      button.setAttribute("aria-pressed", button.dataset.density === which ? "true" : "false");
    });
  }
  densities.forEach(function (button) {
    button.addEventListener("click", function () {
      density(button.dataset.density);
      try { localStorage.setItem("cairns-density", button.dataset.density); } catch (e) {}
    });
  });
  try {
    var saved = localStorage.getItem("cairns-density");
    if (saved) density(saved);
  } catch (e) {}

  // `/` to search, `j` and `k` (or the arrows) to move through what is shown,
  // Enter to open - the link already does that. Escape leaves the search box.
  function visibleRows() {
    return Array.prototype.slice.call(list.querySelectorAll("li:not([hidden]) .row")).filter(
      function (row) { return !row.closest(".day[hidden]"); }
    );
  }
  document.addEventListener("keydown", function (event) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    var typing = /^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName);
    if (typing) {
      if (event.key === "Escape") event.target.blur();
      if (event.key === "ArrowDown" && event.target === box) {
        var first = visibleRows()[0];
        if (first) { event.preventDefault(); first.focus(); }
      }
      return;
    }
    if (event.key === "/" && box) {
      event.preventDefault();
      box.focus();
      return;
    }
    var step = { j: 1, ArrowDown: 1, k: -1, ArrowUp: -1 }[event.key];
    if (!step) return;
    var shown = visibleRows();
    if (!shown.length) return;
    var at = shown.indexOf(document.activeElement);
    var next = at === -1 ? (step > 0 ? 0 : shown.length - 1) : at + step;
    if (next < 0 || next >= shown.length) return;
    event.preventDefault();
    shown[next].focus();
    shown[next].scrollIntoView({ block: "nearest" });
  });

  // Open or closed, on the open questions page. Which rows are shown first is
  // decided by the stylesheet, so the page is laid out right before any of
  // this runs; the switch only flips a class.
  var states = Array.prototype.slice.call(document.querySelectorAll("button[data-state]"));
  var state = "open";
  function showState(which) {
    state = which;
    list.classList.toggle("show-closed", which === "closed");
    states.forEach(function (button) {
      button.setAttribute("aria-pressed", button.dataset.state === which ? "true" : "false");
    });
  }
  states.forEach(function (button) {
    button.addEventListener("click", function () {
      if (state === button.dataset.state) return;
      showState(button.dataset.state);
      syncHash();
      apply();
    });
  });

  // Arriving with a hash, and changing it without leaving the page.
  if (location.hash) fromHash();
  window.addEventListener("hashchange", fromHash);
})();
