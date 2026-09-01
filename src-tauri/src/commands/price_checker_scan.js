// TIQR Manager - Visible Scanner extraction script (2.1.9)
//
// Injected via WebviewWindow::eval_with_callback from
// commands::price_checker_scanner::scan_visible_prices, ONCE per click of
// "Scan Visible Prices" - never on a timer, never auto-scrolling, never
// retried in a loop. marko scrolls/navigates the REAL visible window
// himself; this script's only job is to read what is on screen at the
// moment it's asked to, exactly once, and hand back plain data. See this
// project's PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md for the full design and
// for why the old hidden-WebView/blind-CSS-selector/retry-loop approach
// (price_checker_auto_extract.js, removed in 2.1.9) was abandoned.
//
// LAYERING (marko's spec, "## SCANNING"): this script never depends on a
// single CSS selector list being right. Four layers, in priority order:
//   1. accessibility/UI text  - aria-label / aria-labelledby, when present.
//   2. rendered DOM           - loose, attribute-CONTAINS selectors tuned to
//                                each marketplace's general conventions
//                                (never exact/brittle class names, which
//                                break on the next redeploy).
//   3. visible text           - a marketplace-independent scan of every
//                                visible text node for a money-shaped
//                                pattern. This is the real backbone: it
//                                still finds listings even when every
//                                selector in layer 2 matches nothing, which
//                                is the whole point of not trusting
//                                selectors alone.
//   4. screenshot/visual analysis - NOT implemented. This sandbox's Rust
//                                side has no screenshot-capture or OCR
//                                dependency (see Cargo.toml - no relevant
//                                crate), and adding a full OCR pipeline is
//                                disproportionate to what layers 1-3 already
//                                cover. Honestly reported as a scope
//                                decision in the report, per marko's own
//                                "ak je to technicky možné" (if technically
//                                possible) wording - it deliberately is not,
//                                without a much larger dependency footprint.
//
// Every marketplace reader (StubHub/Vivid Seats/Ticombo) is layers 1+2
// scoped to that site's loose conventions, ALWAYS unioned with layer 3
// (generic) so a reader can never return nothing just because its selectors
// missed - see readStubHub/readVividSeats/readTicombo below, which are thin
// wrappers over scanWithSelectors + readGenericVisibleText.
//
// NEVER invents data: every field on every candidate is either literally
// present in the page's own text/attributes, or omitted (undefined). No
// guessed section/row/quantity, no fabricated listingId.
//
// Returns a JSON STRING (not an object) as this IIFE's completion value -
// deliberately mirrors the throwaway webview_smoke_test.rs finding that
// eval_with_callback's callback receives a plain String, so the Rust side
// (price_checker_scanner.rs) just does serde_json::from_str on it rather
// than relying on any implicit object-to-JSON behavior this Tauri version
// may or may not have.
(function () {
  "use strict";

  // ---------------------------------------------------------------------
  // Money parsing - conservative on purpose. Returns null rather than a
  // guessed amount/currency whenever the text isn't confidently money.
  // ---------------------------------------------------------------------

  var SYMBOL_CURRENCY = {
    "$": "USD",
    "US$": "USD",
    "USD$": "USD",
    "CA$": "CAD",
    "AU$": "AUD",
    "NZ$": "NZD",
    "€": "EUR", // €
    "£": "GBP", // £
    "¥": "JPY", // ¥
    "₩": "KRW", // ₩
    "₹": "INR", // ₹
    "zł": "PLN",
    // Deliberately NOT mapping bare "kr" to a specific currency (SEK, NOK,
    // DKK and ISK all use it) - guessing one would be inventing confidence
    // this parser doesn't actually have. parseMoney still successfully
    // extracts the amount with currency left null/undefined rather than
    // silently mislabeling a Norwegian or Danish price as Swedish.
  };
  var CODE_SET = [
    "USD", "EUR", "GBP", "CAD", "AUD", "NZD", "JPY", "CHF", "SEK", "NOK",
    "DKK", "PLN", "CZK", "HUF", "MXN", "BRL", "ZAR", "SGD", "HKD", "KRW", "INR",
  ];

  // Leading symbol/code: "$123.45", "US$1,234", "EUR 99,90"
  var LEADING_RE = new RegExp(
    "(US\\$|CA\\$|AU\\$|NZ\\$|USD\\$|[$€£¥₩₹]|" +
      CODE_SET.join("|") +
      ")\\s?([0-9][0-9.,\\s ]*[0-9]|[0-9])(?!\\d)"
  );
  // Trailing symbol/code: "123.45 $", "99,90 EUR", "1234 kr"
  var TRAILING_RE = new RegExp(
    "([0-9][0-9.,\\s ]*[0-9]|[0-9])\\s?(kr|zł|[$€£¥₩₹]|" +
      CODE_SET.join("|") +
      ")(?![A-Za-z])"
  );

  function currencyFromToken(token) {
    var t = token.replace(/\s/g, "");
    if (SYMBOL_CURRENCY[t]) return SYMBOL_CURRENCY[t];
    var upper = t.toUpperCase();
    if (CODE_SET.indexOf(upper) !== -1) return upper;
    if (SYMBOL_CURRENCY[token]) return SYMBOL_CURRENCY[token];
    return null;
  }

  // Turns "1,234.56" / "1.234,56" / "1 234,56" / "1234" into a float, or
  // null when the token isn't a plausible amount at all. Handles the
  // classic US-vs-European separator ambiguity with the same heuristic most
  // real-world money parsers use: whichever of ',' or '.' appears LAST is
  // the decimal separator when both are present; when only one appears,
  // it's the decimal separator only if it's followed by exactly 1-2 digits
  // at the very end of the token, otherwise it's a thousands separator.
  function normalizeAmountToken(raw) {
    var token = raw.replace(/[\s ]/g, "");
    if (!/^[0-9.,]+$/.test(token)) return null;
    var lastComma = token.lastIndexOf(",");
    var lastDot = token.lastIndexOf(".");
    var decimalSep = null;
    if (lastComma !== -1 && lastDot !== -1) {
      decimalSep = lastComma > lastDot ? "," : ".";
    } else if (lastComma !== -1) {
      var afterComma = token.length - lastComma - 1;
      decimalSep = afterComma >= 1 && afterComma <= 2 && token.indexOf(",") === lastComma ? "," : null;
    } else if (lastDot !== -1) {
      var afterDot = token.length - lastDot - 1;
      decimalSep = afterDot >= 1 && afterDot <= 2 && token.indexOf(".") === lastDot ? "." : null;
    }
    var normalized;
    if (decimalSep) {
      var thousandsSep = decimalSep === "," ? "." : ",";
      normalized = token.split(thousandsSep).join("");
      normalized = normalized.replace(decimalSep, ".");
    } else {
      // No confident decimal separator - both '.' and ',' (if present) are
      // thousands separators, e.g. "1,234" or "2.500".
      normalized = token.replace(/[.,]/g, "");
    }
    var value = parseFloat(normalized);
    if (!isFinite(value) || value < 0) return null;
    return value;
  }

  // Finds the FIRST confident money pattern in `text`. Returns
  // {cents, currency} or null - never a partial/guessed match.
  function parseMoney(text) {
    if (!text) return null;
    var m = LEADING_RE.exec(text);
    var symbolFirst = true;
    if (!m) {
      m = TRAILING_RE.exec(text);
      symbolFirst = false;
    }
    if (!m) return null;
    var amountToken = symbolFirst ? m[2] : m[1];
    var symbolToken = symbolFirst ? m[1] : m[2];
    var amount = normalizeAmountToken(amountToken);
    if (amount === null) return null;
    var currency = currencyFromToken(symbolToken);
    return { cents: Math.round(amount * 100), currency: currency };
  }

  // ---------------------------------------------------------------------
  // Visibility - "what marko can actually see right now", not just
  // "not display:none". Requires real on-screen intersection with the
  // current viewport so a scan only ever picks up what's on screen at the
  // moment of the click, matching marko's own spec example (scan, scroll,
  // scan again -> two different sets of on-screen listings).
  // ---------------------------------------------------------------------

  function isVisible(el) {
    if (!el || typeof el.getBoundingClientRect !== "function") return false;
    if (el.isConnected === false) return false;
    var style;
    try {
      style = window.getComputedStyle(el);
    } catch (e) {
      return false;
    }
    if (!style || style.display === "none" || style.visibility === "hidden") return false;
    if (parseFloat(style.opacity) === 0) return false;
    var rect = el.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return false;
    var vw = window.innerWidth || document.documentElement.clientWidth || 0;
    var vh = window.innerHeight || document.documentElement.clientHeight || 0;
    if (rect.bottom <= 0 || rect.top >= vh || rect.right <= 0 || rect.left >= vw) return false;
    return true;
  }

  // ---------------------------------------------------------------------
  // Layer 1: accessibility/UI text. aria-label wins when present (it's
  // what a screen reader would say - often a cleaner single string than the
  // visible DOM's scattered child nodes); aria-labelledby is resolved to
  // the referenced element(s)' text; otherwise falls back to trimmed
  // textContent (layer 2/3 territory, but harmless to fall through here).
  // ---------------------------------------------------------------------

  function accessibleText(el) {
    if (!el) return "";
    var ariaLabel = el.getAttribute && el.getAttribute("aria-label");
    if (ariaLabel && ariaLabel.trim()) return ariaLabel.trim();
    var labelledBy = el.getAttribute && el.getAttribute("aria-labelledby");
    if (labelledBy) {
      var parts = labelledBy
        .split(/\s+/)
        .map(function (id) {
          var ref = document.getElementById(id);
          return ref ? ref.textContent.trim() : "";
        })
        .filter(Boolean);
      if (parts.length) return parts.join(" ");
    }
    return (el.textContent || "").trim();
  }

  // ---------------------------------------------------------------------
  // Listing container + nearby context. A "listing container" is the
  // smallest ancestor that plausibly represents one whole listing (list
  // item, article/row role, or a repeated sibling pattern) - used as the
  // scope for section/row/quantity/listingId extraction so those don't leak
  // in from unrelated page text, and as the dedup key within one scan.
  // ---------------------------------------------------------------------

  function findListingContainer(el) {
    var node = el;
    var hops = 0;
    while (node && node !== document.body && hops < 8) {
      var role = node.getAttribute && node.getAttribute("role");
      if (node.tagName === "LI" || role === "listitem" || role === "article" || role === "row") {
        return node;
      }
      var parent = node.parentElement;
      if (parent) {
        var sameTagSiblings = 0;
        // Only ever needs to know "are there >=2" - stop counting the
        // instant that's answered instead of always walking every child.
        // On a large real listings page (many sibling rows under one
        // wrapper, exactly the shape this function is looking for) that
        // turns an O(children) scan into O(1) for every candidate on the
        // page, avoiding an O(n^2) blowup across a whole scan.
        for (var i = 0; i < parent.children.length; i++) {
          if (parent.children[i].tagName === node.tagName) {
            sameTagSiblings++;
            if (sameTagSiblings >= 2) break;
          }
        }
        // A repeated sibling pattern (>=2 same-tag siblings under the same
        // parent) is the classic shape of a rendered listing row - stop
        // here rather than climbing into the shared list wrapper.
        if (sameTagSiblings >= 2) return node;
      }
      node = parent;
      hops++;
    }
    // No clear container found within 8 hops - fall back to a bounded
    // ancestor (3 hops up from the price element itself) rather than the
    // whole page, so context extraction still stays reasonably scoped.
    node = el;
    for (var j = 0; j < 3 && node.parentElement; j++) node = node.parentElement;
    return node;
  }

  var SECTION_RE = /\bsec(?:tion)?\.?\s*[:#]?\s*([A-Za-z0-9\-]{1,15})/i;
  var ROW_RE = /\brow\.?\s*[:#]?\s*([A-Za-z0-9\-]{1,6})/i;
  var QTY_RE = /\b(?:qty|quantity)\.?\s*[:#]?\s*(\d{1,2})\b|\b(\d{1,2})\s*(?:tickets?|seats?)\b|\bx\s?(\d{1,2})\b/i;
  // 2.2.0 (Market Analysis): captures the WHOLE label including its own
  // keyword ("Level 100", not just "100") - marko's own spec, "## TIER
  // PRICING": "Ak marketplace poskytne iný názov tieru, zachovaj jeho
  // názov" (if the marketplace uses a different tier name, keep ITS name).
  // Deliberately covers the common seating-tier vocabulary
  // (level/tier/zone/deck/category) rather than one fixed word - still a
  // loose, best-effort pattern like SECTION_RE/ROW_RE above, not a
  // guarantee; see tierFor's own comment for the second, usually more
  // useful detection path (most real seating-tier UIs show this as a group
  // HEADING above several listing rows, not repeated inline in every row).
  var TIER_RE = /\b((?:level|tier|zone|deck|category)\.?\s*[:#]?\s*[A-Za-z0-9\-]{1,20})/i;

  function nearbyListingContext(containerEl) {
    var text = (containerEl.textContent || "").replace(/\s+/g, " ");
    var out = {};
    var sm = SECTION_RE.exec(text);
    if (sm) out.section = sm[1].trim();
    var rm = ROW_RE.exec(text);
    if (rm) out.row = rm[1].trim();
    var qm = QTY_RE.exec(text);
    if (qm) {
      var qtyStr = qm[1] || qm[2] || qm[3];
      var qty = parseInt(qtyStr, 10);
      if (isFinite(qty) && qty > 0 && qty <= 50) out.quantity = qty;
    }
    var tm = TIER_RE.exec(text);
    if (tm) out.tier = tm[1].trim();
    return out;
  }

  // ---------------------------------------------------------------------
  // Tier/level detection, path 2 (2.2.0, "## TIER PRICING"): most real
  // seating-tier UIs render "Level 100" ONCE as a group heading above a
  // block of listing rows, not repeated inline inside every single row -
  // TIER_RE above only ever catches the inline case. This second path
  // collects every visible heading-shaped element whose text matches
  // TIER_RE ONCE per scan (never per candidate - see scanPage below), then
  // for a given listing container picks the NEAREST one that precedes it in
  // document order, the same "closest heading above this point" association
  // a table of contents or a print stylesheet's running header would use.
  // Best-effort like every other selector in this file - NOT verified
  // against real marketplace markup (this sandbox's network access to
  // StubHub/Vivid Seats/Ticombo is blocked, same limitation as the rest of
  // this script) - a page whose tier grouping doesn't match either this or
  // the inline pattern simply leaves tier undefined, which the Rust side
  // reports honestly as "Unclassified" rather than guessing (marko's own
  // "NEVYMÝŠĽAJ tier mapping").
  // ---------------------------------------------------------------------

  var MAX_TIER_HEADINGS = 60;

  function collectTierHeadings() {
    var out = [];
    var nodes;
    try {
      nodes = document.querySelectorAll('h1, h2, h3, h4, h5, h6, [role="heading"]');
    } catch (e) {
      return out;
    }
    // querySelectorAll already returns nodes in document order, which
    // nearestPrecedingTier below relies on.
    for (var i = 0; i < nodes.length && out.length < MAX_TIER_HEADINGS; i++) {
      var node = nodes[i];
      if (!isVisible(node)) continue;
      var text = accessibleText(node);
      var m = TIER_RE.exec(text);
      if (m) out.push({ node: node, tier: m[1].trim() });
    }
    return out;
  }

  // Cached per scan (this whole script re-runs fresh on every
  // eval_with_callback call - see the module doc comment at the top of this
  // file - so a module-level cache can never leak stale headings into a
  // later, separate scan). Collected lazily so a page with no tier headings
  // at all never pays for the querySelectorAll walk more than once.
  var _tierHeadingsCache = null;
  function tierHeadings() {
    if (_tierHeadingsCache === null) _tierHeadingsCache = collectTierHeadings();
    return _tierHeadingsCache;
  }

  function nearestPrecedingTier(containerEl) {
    var headings = tierHeadings();
    var best = null;
    for (var i = 0; i < headings.length; i++) {
      var rel = headings[i].node.compareDocumentPosition(containerEl);
      var precedes = !!(rel & Node.DOCUMENT_POSITION_FOLLOWING);
      if (precedes) {
        // Headings are in document order, so the LAST one that precedes
        // this container is the nearest one above it - keep overwriting.
        best = headings[i].tier;
        continue;
      }
      // headings[] is in document order, so once one heading no longer
      // precedes the container, every later heading (further down the
      // document) can't precede it either - safe to stop scanning early,
      // whether or not a preceding one was already found.
      break;
    }
    return best;
  }

  function tierFor(containerEl, inlineTier) {
    if (inlineTier) return inlineTier;
    if (!tierHeadings().length) return undefined;
    return nearestPrecedingTier(containerEl) || undefined;
  }

  // Best-effort element identity - checked on the container first (a real
  // listing id almost always lives on the row wrapper, not the price span
  // inside it), then a couple of ancestors above it. Most real listing
  // elements won't have any of these, so returning undefined is the normal
  // case, not a failure - see NormalizedListing::listing_id's own doc
  // comment (models.rs).
  var ID_ATTRS = ["data-listing-id", "data-listingid", "data-id", "data-key", "data-testid", "id"];
  function listingIdFor(containerEl) {
    var node = containerEl;
    var hops = 0;
    while (node && hops < 3) {
      for (var i = 0; i < ID_ATTRS.length; i++) {
        var v = node.getAttribute && node.getAttribute(ID_ATTRS[i]);
        if (v && v.trim() && v.trim().length <= 120) return v.trim();
      }
      node = node.parentElement;
      hops++;
    }
    return undefined;
  }

  // ---------------------------------------------------------------------
  // Candidate assembly + within-scan dedup. Two different layers (a
  // selector match and the generic text scan) can easily land on the same
  // visual listing - deduped here by listing-container identity so the
  // RESULT of one scan never double-counts a single on-screen listing.
  // Cross-scan dedup (the same listing seen again after scrolling back) is
  // a separate, session-level concern handled in Rust
  // (commands::price_checker_scanner::fingerprint_for).
  // ---------------------------------------------------------------------

  function candidateFrom(priceEl, marketplace) {
    if (!isVisible(priceEl)) return null;
    var text = accessibleText(priceEl) || (priceEl.textContent || "");
    var money = parseMoney(text);
    if (!money) return null;
    var container = findListingContainer(priceEl);
    var ctx = nearbyListingContext(container);
    return {
      _container: container,
      priceCents: money.cents,
      currency: money.currency || undefined,
      section: ctx.section,
      row: ctx.row,
      quantity: ctx.quantity,
      tier: tierFor(container, ctx.tier),
      listingId: listingIdFor(container),
      marketplace: marketplace,
    };
  }

  function dedupeByContainer(candidates) {
    // A Set gives O(1) "have I seen this container" lookups - with an
    // array + indexOf, a page with many real listings turns this into an
    // O(n^2) pass over the whole candidate list.
    var seen = typeof Set !== "undefined" ? new Set() : null;
    var seenArr = seen ? null : [];
    var out = [];
    for (var i = 0; i < candidates.length; i++) {
      var c = candidates[i];
      if (!c) continue;
      var already = seen ? seen.has(c._container) : seenArr.indexOf(c._container) !== -1;
      if (already) continue;
      if (seen) seen.add(c._container);
      else seenArr.push(c._container);
      delete c._container;
      out.push(c);
    }
    return out;
  }

  // ---------------------------------------------------------------------
  // Layer 3 (generic, marketplace-independent): walk every visible text
  // node, test it for a money pattern. This is the layer that guarantees a
  // result even when a site's markup doesn't match ANY known selector -
  // the actual point of not depending on CSS selectors alone.
  // ---------------------------------------------------------------------

  function readGenericVisibleText(marketplace, maxMatches) {
    var out = [];
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null);
    var scanned = 0;
    var node;
    while ((node = walker.nextNode()) && out.length < maxMatches && scanned < 20000) {
      scanned++;
      var text = node.nodeValue;
      if (!text || !/[0-9]/.test(text)) continue;
      var parent = node.parentElement;
      if (!parent) continue;
      var money = parseMoney(text);
      if (!money) continue;
      var c = candidateFrom(parent, marketplace);
      if (c) out.push(c);
    }
    return { candidates: out, elementsScanned: scanned };
  }

  // ---------------------------------------------------------------------
  // Layer 2: loose, attribute-CONTAINS selectors per marketplace. Never an
  // exact class name (those break the moment a site redeploys its CSS) -
  // always a case-insensitive "contains" match against class/data-testid/
  // aria-label for words that plausibly mark a listing/price row. These are
  // reasoned best-effort guesses tuned to each site's general conventions,
  // NOT verified against the live marketplaces - this sandbox's network
  // access to stubhub.com/vividseats.com/ticombo.com is blocked (see the
  // report's own verification section), so they're deliberately written to
  // degrade harmlessly (readGenericVisibleText below still runs regardless
  // of whether any of these match) rather than to be trusted alone.
  // ---------------------------------------------------------------------

  // Same spirit as readGenericVisibleText's own maxMatches/scanned caps -
  // without one, a real page with an unusually large number of visible
  // listings (or several of these loose selectors all matching the same
  // large set of elements) has no ceiling on how many elements get
  // processed here, each walking findListingContainer/nearbyListingContext.
  // 500 is generous headroom above what a real single visible-viewport-ish
  // scan should ever need - candidates past this point are still available
  // on the next scan.
  var MAX_SELECTOR_ELEMENTS = 500;

  function scanWithSelectors(selectors, marketplace) {
    var out = [];
    var hits = 0;
    for (var i = 0; i < selectors.length; i++) {
      if (hits >= MAX_SELECTOR_ELEMENTS) break;
      var found;
      try {
        found = document.querySelectorAll(selectors[i]);
      } catch (e) {
        continue;
      }
      for (var j = 0; j < found.length && hits < MAX_SELECTOR_ELEMENTS; j++) {
        hits++;
        var c = candidateFrom(found[j], marketplace);
        if (c) out.push(c);
      }
    }
    return { candidates: out, selectorHits: hits };
  }

  var LISTING_PRICE_SELECTORS = [
    '[data-testid*="price" i]',
    '[data-testid*="listing" i]',
    '[class*="listing" i] [class*="price" i]',
    '[class*="ticket" i] [class*="price" i]',
    '[aria-label*="price" i]',
    '[aria-label*="ticket" i]',
  ];

  function readStubHub() {
    return scanWithSelectors(LISTING_PRICE_SELECTORS, "stubhub");
  }
  function readVividSeats() {
    return scanWithSelectors(LISTING_PRICE_SELECTORS, "vividseats");
  }
  function readTicombo() {
    return scanWithSelectors(LISTING_PRICE_SELECTORS, "ticombo");
  }

  function hostFamily(hostname) {
    var h = (hostname || "").toLowerCase();
    if (h.indexOf("stubhub") !== -1) return "stubhub";
    if (h.indexOf("vividseats") !== -1) return "vividseats";
    if (h.indexOf("ticombo") !== -1) return "ticombo";
    return "generic";
  }

  // ---------------------------------------------------------------------
  // Blocked/challenge detection - DETECTION ONLY, never a bypass attempt
  // (marko's explicit "## SECURITY": no CAPTCHA/anti-bot bypass, ever). Just
  // enough to tell marko "Unable to read automatically" honestly instead of
  // silently returning zero listings from what's actually a challenge page.
  // ---------------------------------------------------------------------

  function detectBlocked() {
    var title = (document.title || "").toLowerCase();
    var bodyText = ((document.body && document.body.innerText) || "").slice(0, 2000).toLowerCase();
    var markers = [
      "just a moment",
      "checking your browser",
      "attention required",
      "access denied",
      "are you a human",
      "verify you are human",
      "unusual traffic",
      "please verify you are a human",
    ];
    for (var i = 0; i < markers.length; i++) {
      if (title.indexOf(markers[i]) !== -1 || bodyText.indexOf(markers[i]) !== -1) return markers[i];
    }
    if (document.querySelector('iframe[src*="recaptcha" i], iframe[title*="challenge" i], #challenge-running')) {
      return "challenge widget detected";
    }
    return null;
  }

  // ---------------------------------------------------------------------
  // Orchestration
  // ---------------------------------------------------------------------

  function scanPage() {
    var hostname = location.hostname || "";
    var marketplace = hostFamily(hostname);
    var blockedReason = detectBlocked();

    var layered;
    try {
      if (marketplace === "stubhub") layered = readStubHub();
      else if (marketplace === "vividseats") layered = readVividSeats();
      else if (marketplace === "ticombo") layered = readTicombo();
      else layered = { candidates: [], selectorHits: 0 };
    } catch (e) {
      layered = { candidates: [], selectorHits: 0, readerError: String((e && e.message) || e) };
    }

    var generic;
    try {
      generic = readGenericVisibleText(marketplace, 400);
    } catch (e) {
      generic = { candidates: [], elementsScanned: 0, readerError: String((e && e.message) || e) };
    }

    var candidates = dedupeByContainer((layered.candidates || []).concat(generic.candidates || []));

    return {
      ok: true,
      marketplace: marketplace,
      hostname: hostname,
      url: location.href,
      title: document.title || "",
      blocked: !!blockedReason,
      blockedReason: blockedReason || undefined,
      candidates: candidates,
      diagnostics: {
        selectorHits: layered.selectorHits || 0,
        selectorLayerError: layered.readerError,
        genericElementsScanned: generic.elementsScanned || 0,
        genericLayerError: generic.readerError,
        totalBeforeDedup: (layered.candidates || []).length + (generic.candidates || []).length,
        totalAfterDedup: candidates.length,
      },
    };
  }

  var payload;
  try {
    payload = scanPage();
  } catch (e) {
    payload = {
      ok: false,
      marketplace: hostFamily(location.hostname || ""),
      hostname: location.hostname || "",
      url: location.href,
      title: document.title || "",
      blocked: false,
      candidates: [],
      diagnostics: { fatalError: String((e && e.message) || e) },
    };
  }

  return JSON.stringify(payload);
})()
