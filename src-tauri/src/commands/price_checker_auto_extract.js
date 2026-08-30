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
  if (prices.length === 0) {
    var tables = document.querySelectorAll("table");
    tables.forEach(function (table) {
      var headerEl = table.querySelector("thead") || table;
      var headerText = (headerEl.textContent || "").toLowerCase();
      if (headerText.indexOf("section") === -1 && headerText.indexOf("price") === -1) return;
      var rows = table.querySelectorAll("tbody tr, tr");
      rows.forEach(function (row) {
        var cells = row.querySelectorAll("td");
        if (cells.length < 3) return;
        for (var i = 0; i < cells.length; i++) {
          var m = /([$€£])\s?([\d.,]+)/.exec(cells[i].textContent || "");
          if (m) {
            var val = parseFloat(m[2].replace(/,/g, ""));
            if (isFinite(val) && val > 0) {
              prices.push(val);
              if (!currency) currency = m[1] === "$" ? "USD" : m[1] === "€" ? "EUR" : "GBP";
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

  return JSON.stringify({ prices: prices, currency: currency, blocked: false });
})();
