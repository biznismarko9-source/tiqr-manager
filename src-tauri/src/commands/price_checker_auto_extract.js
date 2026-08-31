(function () {
  var titleLower = (document.title || "").toLowerCase();
  var blocked = ["just a moment", "attention required", "access denied", "pardon our interruption", "are you a human", "verify you are human"].some(
    function (s) { return titleLower.indexOf(s) !== -1; }
  );
  if (blocked) {
    return JSON.stringify({ prices: [], currency: null, blocked: true });
  }

  var prices = [];
  var currency = null;

  // --- Pass 1: schema.org JSON-LD (Offer[] or AggregateOffer) ---
  document.querySelectorAll('script[type="application/ld+json"]').forEach(function (s) {
    try {
      var data = JSON.parse(s.textContent);
      var items = Array.isArray(data) ? data : [data];
      items.forEach(function (item) { collectOffers(item); });
    } catch (e) { /* malformed block - skip, don't guess */ }
  });

  function collectOffers(node) {
    if (!node || typeof node !== "object") return;
    if (node.offers) {
      var offers = Array.isArray(node.offers) ? node.offers : [node.offers];
      offers.forEach(function (o) {
        if (!o || typeof o !== "object") return;
        var type = String(o["@type"] || "").toLowerCase();
        if (type.indexOf("aggregateoffer") !== -1) {
          if (o.lowPrice) pushPrice(o.lowPrice, o.priceCurrency);
          if (o.highPrice) pushPrice(o.highPrice, o.priceCurrency);
        } else if (o.price) {
          pushPrice(o.price, o.priceCurrency);
        }
      });
    }
    Object.keys(node).forEach(function (k) {
      var v = node[k];
      if (v && typeof v === "object") collectOffers(v);
    });
  }

  function pushPrice(raw, cur) {
    var n = typeof raw === "number" ? raw : parseFloat(raw);
    if (isFinite(n) && n > 0) {
      prices.push(n);
      if (!currency && cur) currency = cur;
    }
  }

  // --- Pass 2: HTML table shaped like Section/Row/Price (Vivid-Seats-style) ---
  var listings = [];
  if (prices.length === 0) {
    var tables = document.querySelectorAll("table");
    tables.forEach(function (table) {
      var headerEl = table.querySelector("thead") || table;
      var headerCells = Array.prototype.map.call(headerEl.querySelectorAll("th"), function (th) { return (th.textContent || "").trim().toLowerCase(); });
      var headerText = (headerEl.textContent || "").toLowerCase();
      if (headerText.indexOf("section") === -1 && headerText.indexOf("price") === -1) return;
      // marko's spec ("EXTRACTION") wants real section/row/quantity, not
      // just bare prices, wherever the page actually states them - find
      // each column's index (if the table has one) rather than assuming a
      // fixed order, so this keeps working if Vivid Seats ever reorders
      // its own columns.
      var sectionCol = headerCells.indexOf("section");
      var rowCol = headerCells.indexOf("row");
      var qtyCol = headerCells.findIndex(function (h) { return h.indexOf("qty") !== -1 || h.indexOf("quantity") !== -1; });

      var rows = table.querySelectorAll("tbody tr, tr");
      rows.forEach(function (row) {
        var cells = row.querySelectorAll("td");
        if (cells.length < 3) return;
        var cellText = function (i) { return i >= 0 && i < cells.length ? (cells[i].textContent || "").trim() : null; };
        for (var i = 0; i < cells.length; i++) {
          var m = /([$€£])\s?([\d.,]+)/.exec(cells[i].textContent || "");
          if (m) {
            var val = parseFloat(m[2].replace(/,/g, ""));
            if (isFinite(val) && val > 0) {
              var cur = m[1] === "$" ? "USD" : m[1] === "€" ? "EUR" : "GBP";
              prices.push(val);
              if (!currency) currency = cur;
              var qtyRaw = qtyCol >= 0 ? cellText(qtyCol) : null;
              var qtyNum = qtyRaw ? parseInt(qtyRaw.replace(/[^\d]/g, ""), 10) : NaN;
              listings.push({
                price: val,
                currency: cur,
                section: sectionCol >= 0 ? cellText(sectionCol) : cellText(0),
                row: rowCol >= 0 ? cellText(rowCol) : cellText(1),
                quantity: isFinite(qtyNum) ? qtyNum : null,
              });
            }
            break;
          }
        }
      });
    });
  }

  // --- Pass 3: Open Graph og:price:amount meta tag ---
  if (prices.length === 0) {
    var amountMeta = document.querySelector('meta[property="og:price:amount"]');
    var amount = amountMeta ? parseFloat(amountMeta.getAttribute("content")) : NaN;
    if (isFinite(amount) && amount > 0) {
      prices.push(amount);
      var curMeta = document.querySelector('meta[property="og:price:currency"]');
      currency = (curMeta && curMeta.getAttribute("content")) || currency;
    }
  }

  return JSON.stringify({ prices: prices, currency: currency, blocked: false, listings: listings });
})();
