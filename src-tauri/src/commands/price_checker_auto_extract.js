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

  // --- Pass 4: generic visible-text scan (last resort) ---
  // Mirrors the exact same "number directly next to a currency marker"
  // logic already proven in the EXISTING manual-paste flow
  // (src/lib/priceParse.ts's own CONFIDENT_TOKEN) - this only ever finds
  // what marko copying the same visible text and pasting it into the
  // existing box would ALSO have found, not a newly-invented heuristic.
  // Deliberately blind to the surrounding HTML (works whether the real
  // listings sit inside a <table>, a <div>-based grid, or anything else) -
  // Pass 2 above only recognizes a semantic <table>, which some
  // marketplaces may not actually use for their listings even when Pass 2
  // was originally confirmed against one that did.
  var diagnostics = { tableCount: document.querySelectorAll("table").length, textSample: null, aiText: null };
  if (prices.length === 0) {
    var bodyText = document.body ? (document.body.innerText || document.body.textContent || "") : "";
    diagnostics.textSample = bodyText.slice(0, 600);
    // 2.1.6: a longer slice of the exact same visible text, for the new
    // AI-assisted extraction fallback (commands/price_checker_auto.rs) to
    // read when Pass 4 below still doesn't find its required 2 confident
    // matches - never computed/sent anywhere unless prices stay empty here,
    // same "only when everything else already failed" spirit the fallback
    // itself follows. 8000 chars is generous for a real listings section
    // without sending the whole page - same reasoning as textSample's own
    // 600-char cap, just sized for a model to read instead of a human.
    diagnostics.aiText = bodyText.slice(0, 8000);
    var CONFIDENT = /([€$£])\s?(\d[\d.,]*\d|\d)|(\d[\d.,]*\d|\d)\s?([€$£])(?!\d)/g;
    var genericFound = [];
    var gm;
    while ((gm = CONFIDENT.exec(bodyText)) !== null) {
      var sym = gm[1] || gm[4];
      var numRaw = gm[2] || gm[3];
      var val = parseFloat(numRaw.replace(/,/g, ""));
      if (isFinite(val) && val > 0 && val < 100000) genericFound.push({ val: val, sym: sym });
    }
    // Requires at least 2 matches - a single stray price-looking number
    // elsewhere on the page (nav, footer, an unrelated widget) shouldn't
    // count as "found real listings"; a real listings section always shows
    // several, same caution already applied to Pass 2/3 above (a header
    // match / an actual meta tag, never a single bare guess).
    if (genericFound.length >= 2) {
      genericFound.forEach(function (f) {
        prices.push(f.val);
        if (!currency) currency = f.sym === "$" ? "USD" : f.sym === "€" ? "EUR" : "GBP";
      });
    }
  }

  return JSON.stringify({ prices: prices, currency: currency, blocked: false, listings: listings, diagnostics: diagnostics, title: document.title });
})();
