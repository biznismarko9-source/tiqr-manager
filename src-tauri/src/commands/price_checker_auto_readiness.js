(function () {
  // 2.1.8 - mirrors price_checker_auto_extract.js's own per-marketplace
  // dispatch (see that file's own top comment for why) instead of one
  // generic "is there a $ anywhere" signal, and adds a gentle incremental
  // scroll step so lazy-loaded listings actually get a chance to render
  // before the extraction pass runs - marko's spec sections 4/8. Runs every
  // POLL_INTERVAL tick from poll_then_extract (Rust side), so this must
  // stay cheap - no full extraction work here, only "is it worth trying a
  // real extraction pass yet".

  var titleLower = (document.title || "").toLowerCase();
  var blocked = ["just a moment", "attention required", "access denied", "pardon our interruption", "are you a human", "verify you are human"].some(
    function (s) { return titleLower.indexOf(s) !== -1; }
  );

  function hostFamily() {
    var h = (location.hostname || "").toLowerCase();
    if (h.indexOf("stubhub") !== -1) return "stubhub";
    if (h.indexOf("vividseats") !== -1) return "vividseats";
    if (h.indexOf("ticombo") !== -1) return "ticombo";
    return "generic";
  }

  function textOf(el) { return el ? (el.textContent || "").trim() : ""; }

  // Mirrors the extractor's own layered selectors exactly - "ready" must
  // mean "the extractor would actually find candidates right now", not a
  // looser guess, or the poll loop could stop waiting just before the real
  // extraction pass runs and still come up empty.
  function anyMatch(selectors) {
    for (var i = 0; i < selectors.length; i++) {
      try {
        if (document.querySelectorAll(selectors[i]).length > 0) return true;
      } catch (e) { /* skip an unsupported selector, never fail readiness because of it */ }
    }
    return false;
  }

  function stubHubReady() {
    return anyMatch([
      '[data-testid*="listing" i]', '[data-testid*="ticket" i]', '[data-testid*="offer" i]',
      '[class*="listing" i][class*="row" i]', '[class*="listing" i][class*="card" i]', 'li[role="listitem"]',
    ]);
  }

  function vividSeatsReady() {
    var tableReady = false;
    document.querySelectorAll("table").forEach(function (table) {
      if (tableReady) return;
      var headerEl = table.querySelector("thead") || table;
      var headerText = textOf(headerEl).toLowerCase();
      if (headerText.indexOf("section") !== -1 || headerText.indexOf("price") !== -1) tableReady = true;
    });
    return tableReady || anyMatch(['[data-testid*="listing" i]', '[data-testid*="ticket" i]', '[class*="listing" i][class*="card" i]', 'li[role="listitem"]']);
  }

  function ticomboReady() {
    var ogPrice = document.querySelector('meta[property="og:price:amount"]');
    if (ogPrice && ogPrice.getAttribute("content")) return true;
    return anyMatch(['[data-testid*="price" i]', '[data-testid*="listing" i]', '[class*="price-card" i]', '[class*="listing" i][class*="item" i]']);
  }

  // Unchanged generic signal (JSON-LD offers present / a Section-or-Price
  // table / an og:price meta tag / at least 2 confident currency-adjacent
  // matches in the first 3000 chars of visible text) - still what governs
  // readiness for any marketplace marko adds beyond the three with a
  // dedicated reader above.
  function genericReady() {
    var hasJsonLdOffers = false;
    document.querySelectorAll('script[type="application/ld+json"]').forEach(function (s) {
      if (/"offers"|"price"/.test(s.textContent || "")) hasJsonLdOffers = true;
    });
    var hasPriceTable = false;
    document.querySelectorAll("table").forEach(function (table) {
      if (hasPriceTable) return;
      var headerEl = table.querySelector("thead") || table;
      var headerText = textOf(headerEl).toLowerCase();
      if (headerText.indexOf("section") !== -1 || headerText.indexOf("price") !== -1) hasPriceTable = true;
    });
    var ogPrice = document.querySelector('meta[property="og:price:amount"]');
    var hasOgPrice = !!(ogPrice && ogPrice.getAttribute("content"));
    var hasGenericPrices = false;
    if (!hasJsonLdOffers && !hasPriceTable && !hasOgPrice) {
      var bodyText = document.body ? (document.body.innerText || document.body.textContent || "").slice(0, 3000) : "";
      var count = (bodyText.match(/[€$£]\s?\d|\d\s?[€$£](?!\d)/g) || []).length;
      hasGenericPrices = count >= 2;
    }
    return hasJsonLdOffers || hasPriceTable || hasOgPrice || hasGenericPrices;
  }

  var family = hostFamily();
  var ready = blocked || (
    family === "stubhub" ? stubHubReady() :
    family === "vividseats" ? vividSeatsReady() :
    family === "ticombo" ? ticomboReady() :
    genericReady()
  );

  // Gentle incremental scroll (marko's spec section 4): only bother once
  // not yet ready (no point disturbing a page that already looks readable),
  // capped at 6 steps of one viewport height each, state kept on `window`
  // since this whole script re-runs fresh every poll tick and has no other
  // way to remember how far it already scrolled. Never a big jump, never
  // scripted mouse/click events - exactly "window.scrollTo(...), wait for
  // new content" as specified, nothing that mimics human interaction beyond
  // that.
  if (!ready && !blocked) {
    var step = window.__tiqrScrollStep || 0;
    if (step < 6 && document.body && document.body.scrollHeight > window.innerHeight) {
      window.scrollTo(0, Math.min(document.body.scrollHeight, (step + 1) * window.innerHeight));
      window.__tiqrScrollStep = step + 1;
    }
  }

  return JSON.stringify({ ready: ready, blocked: blocked });
})();
