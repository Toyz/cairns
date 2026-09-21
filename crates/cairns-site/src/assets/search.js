// Filtering for the index: area chips and a text box over the whole log.
//
// The entry list is in the HTML already, so it reads fine with this file
// blocked or still loading - filtering is the enhancement, not the page.
(function () {
  var list = document.getElementById("entries");
  if (!list) return;

  var rows = Array.prototype.slice.call(list.querySelectorAll("li"));
  var box = document.getElementById("q");
  var chips = Array.prototype.slice.call(document.querySelectorAll(".chip[data-area]"));
  var status = document.getElementById("status");
  var picked = new Set();
  var text = null; // number -> haystack, fetched on demand

  function load() {
    if (text) return Promise.resolve(text);
    return fetch("search.json")
      .then(function (r) { return r.json(); })
      .then(function (rows) {
        text = new Map();
        rows.forEach(function (row) { text.set(String(row.n), row.t.toLowerCase()); });
        return text;
      })
      .catch(function () {
        // Offline, or opened as a file:// page. Titles still filter.
        text = new Map();
        rows.forEach(function (row) {
          text.set(row.dataset.n, (row.textContent || "").toLowerCase());
        });
        return text;
      });
  }

  function apply() {
    var query = (box && box.value || "").trim().toLowerCase();
    var shown = 0;

    rows.forEach(function (row) {
      var areas = (row.dataset.areas || "").split(" ");
      var byArea = picked.size === 0 || areas.some(function (a) { return picked.has(a); });
      var hay = text ? text.get(row.dataset.n) || "" : (row.textContent || "").toLowerCase();
      var byText = query === "" || hay.indexOf(query) !== -1;
      var show = byArea && byText;
      row.hidden = !show;
      if (show) shown++;
    });

    if (status) {
      var filtered = picked.size > 0 || query !== "";
      status.textContent = filtered ? shown + " of " + rows.length + " entries" : "";
    }
  }

  function syncHash() {
    var areas = Array.from(picked);
    var hash = areas.length ? "#area=" + areas.join(",") : "";
    history.replaceState(null, "", location.pathname + location.search + hash);
  }

  chips.forEach(function (chip) {
    chip.addEventListener("click", function () {
      var area = chip.dataset.area;
      if (picked.has(area)) picked.delete(area); else picked.add(area);
      chip.setAttribute("aria-pressed", picked.has(area) ? "true" : "false");
      syncHash();
      apply();
    });
  });

  if (box) {
    box.addEventListener("focus", function () { load().then(apply); }, { once: true });
    box.addEventListener("input", function () { load().then(apply); });
  }

  var fromHash = /#area=([^&]+)/.exec(location.hash);
  if (fromHash) {
    decodeURIComponent(fromHash[1]).split(",").forEach(function (area) { picked.add(area); });
    chips.forEach(function (chip) {
      if (picked.has(chip.dataset.area)) chip.setAttribute("aria-pressed", "true");
    });
    apply();
  }
})();
