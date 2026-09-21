// Filtering and ordering for the index.
//
// The list is in the HTML already, newest first, so the page reads correctly
// with this file blocked or still loading. Everything here is enhancement.
(function () {
  var list = document.getElementById("entries");
  if (!list) return;

  var rows = Array.prototype.slice.call(list.querySelectorAll("li")); // newest first
  var box = document.getElementById("q");
  var chips = Array.prototype.slice.call(document.querySelectorAll(".chip[data-area]"));
  var sorters = Array.prototype.slice.call(document.querySelectorAll(".sort button[data-sort]"));
  var status = document.getElementById("status");

  var picked = new Set();
  var order = "new";
  var text = null; // number -> haystack, fetched on demand

  function load() {
    if (text) return Promise.resolve(text);
    return fetch("search.json")
      .then(function (r) { return r.json(); })
      .then(function (data) {
        text = new Map();
        data.forEach(function (row) { text.set(String(row.n), row.t.toLowerCase()); });
        return text;
      })
      .catch(function () {
        // Offline, or opened as a file:// page. Titles and summaries still filter.
        text = new Map();
        rows.forEach(function (row) {
          text.set(row.dataset.n, (row.textContent || "").toLowerCase());
        });
        return text;
      });
  }

  function reorder() {
    var wanted = order === "old" ? rows.slice().reverse() : rows;
    var fragment = document.createDocumentFragment();
    wanted.forEach(function (row) { fragment.appendChild(row); });
    list.appendChild(fragment);
  }

  function apply() {
    var query = ((box && box.value) || "").trim().toLowerCase();
    var shown = 0;

    rows.forEach(function (row) {
      var areas = (row.dataset.areas || "").split(" ");
      var byArea = picked.size === 0 || areas.some(function (a) { return picked.has(a); });
      var hay = text ? text.get(row.dataset.n) || "" : (row.textContent || "").toLowerCase();
      var show = byArea && (query === "" || hay.indexOf(query) !== -1);
      row.hidden = !show;
      if (show) shown++;
    });

    if (status) {
      status.textContent =
        picked.size > 0 || query !== "" ? shown + " of " + rows.length + " entries" : "";
    }
  }

  function syncHash() {
    var parts = [];
    if (order === "old") parts.push("sort=old");
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

  // A shared link carries its order and its filters.
  var hash = location.hash.replace(/^#/, "");
  if (hash) {
    hash.split("&").forEach(function (part) {
      var pair = part.split("=");
      if (pair[0] === "sort" && pair[1] === "old") order = "old";
      if (pair[0] === "area" && pair[1]) {
        decodeURIComponent(pair[1]).split(",").forEach(function (area) { picked.add(area); });
      }
    });
    sorters.forEach(function (button) {
      button.setAttribute("aria-pressed", button.dataset.sort === order ? "true" : "false");
    });
    chips.forEach(function (chip) {
      if (picked.has(chip.dataset.area)) chip.setAttribute("aria-pressed", "true");
    });
    if (order === "old") reorder();
    apply();
  }
})();
