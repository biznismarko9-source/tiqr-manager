(function () {
  var titleLower = (document.title || "").toLowerCase();
  var blocked = ["just a moment", "attention required", "access denied", "pardon our interruption", "are you a human", "verify you are human"].some(
    function (s) { return titleLower.indexOf(s) !== -1; }
  );

  var hasJsonLdOffers = false;
  document.querySelectorAll('script[type="application/ld+json"]').forEach(function (s) {
    if (/"offers"|"price"/.test(s.textContent || "")) hasJsonLdOffers = true;
  });

  // Mirrors price_checker_auto_extract.js's own Pass 2 gate exactly (a real
  // <table> whose header mentions "section" or "price") rather than a loose
  // "is there a currency symbol anywhere on the page" scan - marketplace
  // pages routinely show a "$" in marketing copy, nav, or a price filter
  // well before the actual listings table renders, and a looser check here
  // would call the page "ready" and stop polling before that table exists,
  // returning a premature "unable_to_read" on pages that would have worked.
  var hasPriceTable = false;
  document.querySelectorAll("table").forEach(function (table) {
    if (hasPriceTable) return;
    var headerEl = table.querySelector("thead") || table;
    var headerText = (headerEl.textContent || "").toLowerCase();
    if (headerText.indexOf("section") !== -1 || headerText.indexOf("price") !== -1) hasPriceTable = true;
  });

  var ogPrice = document.querySelector('meta[property="og:price:amount"]');
  var hasOgPrice = !!(ogPrice && ogPrice.getAttribute("content"));

  // Mirrors EXTRACT_JS's own Pass 4 (generic visible-text currency-adjacent
  // scan) so the polling loop can detect "ready" as soon as this signal
  // appears too, instead of always running out the full budget before the
  // one best-effort final extraction gets a chance to find it.
  var hasGenericPrices = false;
  if (!hasJsonLdOffers && !hasPriceTable && !hasOgPrice) {
    var bodyText = document.body ? (document.body.innerText || document.body.textContent || "").slice(0, 3000) : "";
    var CONFIDENT_COUNT = (bodyText.match(/[€$£]\s?\d|\d\s?[€$£](?!\d)/g) || []).length;
    hasGenericPrices = CONFIDENT_COUNT >= 2;
  }

  return JSON.stringify({ ready: blocked || hasJsonLdOffers || hasPriceTable || hasOgPrice || hasGenericPrices, blocked: blocked });
})();
