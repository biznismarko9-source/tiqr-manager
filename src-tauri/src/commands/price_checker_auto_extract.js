(function () {
  // 2.1.8 - REAL DOM READER REWRITE. marko's own spec: stop using one
  // generic parser for all three marketplaces; give each a real reader that
  // inspects real rendered DOM (not just document.body.innerText), and be
  // honest about confidence - a bare price is not the same as a real
  // listing (price + section/row/seat context). See PRICE-CHECKER-REAL-DOM-
  // READER-REPORT.md for the full design and, importantly, what could and
  // could not be confirmed against the real live sites from this sandbox
  // (network to stubhub.com/vividseats.com/ticombo.com/viagogo.com is
  // blocked here - re-checked fresh before writing this file, still
  // blocked). The selectors below are best-effort, grounded in patterns
  // genuinely common across large React/Vue-driven marketplace sites
  // (data-testid conventions, ARIA list/listitem roles, class-name tokens),
  // layered so a miss on one layer falls through to the next - NOT
  // confirmed against today's real DOM. The rich diagnostics this file now
  // always returns exist specifically so a real run against a real page
  // that still finds nothing is immediately fixable from what marko can
  // paste back, the same way this exact rewrite started.

  var MAX_CANDIDATES = 300; // hard cap - a runaway selector match must never make this slow or huge
  var DOM_SNAPSHOT_MAX_CHARS = 4000;
  // Diagnostics-only side channel - see readCandidates's own comment.
  var lastCandidateContainer = null;

  // ---------------------------------------------------------------------
  // Shared helpers
  // ---------------------------------------------------------------------

  function textOf(el) {
    return el ? (el.textContent || "").trim() : "";
  }

  // Like textOf(), but inserts a boundary space at every element edge before
  // collapsing whitespace - found necessary while building the jsdom test
  // harness for this file (2.1.8). Plain .textContent concatenates sibling
  // elements with NOTHING between them when the source markup has no literal
  // whitespace text node separating them - exactly how JSX compiles adjacent
  // elements (`<span>{section}</span><span>{row}</span>`), which is a common
  // real-world shape for a listing row's fields. Without this, e.g. a "Row
  // 12" field immediately followed by a "2 tickets" field reads as one
  // digit run "122 tickets" and nearbyListingContext below silently reports
  // a quantity of 122 instead of 2 - a wrong NUMBER, not just messy text,
  // which is exactly the kind of thing marko's "never invent data" spec
  // section warns against. Only used for the section/row/quantity CONTEXT
  // scan below, never for price parsing itself - parseMoney's own currency-
  // symbol-anchored pattern doesn't have this ambiguity (see readCandidates).
  function textWithGaps(el) {
    if (!el) return "";
    var parts = [];
    (function walk(node) {
      if (node.nodeType === 3) {
        parts.push(node.nodeValue);
      } else if (node.nodeType === 1) {
        for (var i = 0; i < node.childNodes.length; i++) walk(node.childNodes[i]);
        parts.push(" "); // boundary gap after every element's own subtree
      }
    })(el);
    return parts.join("").replace(/\s+/g, " ").trim();
  }

  // One money token: currency symbol immediately before/after a number.
  // Same confident shape src/lib/priceParse.ts (the existing manual-paste
  // parser) and the pre-2.1.8 generic pass both already used - deliberately
  // not loosened, a real ticket price is always shown right next to its
  // currency marker on every marketplace looked at so far.
  var MONEY_RE = /([€$£])\s?(\d[\d.,]*\d|\d)|(\d[\d.,]*\d|\d)\s?([€$£])(?!\d)/;

  // Ported verbatim (same algorithm, plain JS instead of TS) from
  // src/lib/priceParse.ts's own normalizeAmountToken - found missing here
  // while adversarially reviewing this file (2.1.8): the code below this
  // used to just do `parseFloat(numRaw.replace(/,/g, ""))`, which silently
  // DELETES commas rather than treating them as a locale-dependent
  // separator. That's only correct for the US/UK "1,234.56" convention - on
  // a Eurozone page (marko is in Slovakia; Viagogo and European StubHub/
  // Vivid Seats/Ticombo pages commonly show "234,56 €"), it silently turned
  // €234.56 into 23456 and €1.234,56 into 1.23456 - a 100x-1000x error with
  // no warning, no rejection, full "confident" status. This looks at the
  // LAST separator and how many digits follow it to correctly tell decimal
  // apart from thousands-grouping regardless of locale, exactly like the
  // manual-paste parser already did - this file just hadn't reused it.
  function normalizeAmountToken(raw) {
    var s = raw.replace(/\s/g, "");
    var lastSepIdx = Math.max(s.lastIndexOf("."), s.lastIndexOf(","));
    if (lastSepIdx === -1) {
      var wholeOnly = parseInt(s, 10);
      return isFinite(wholeOnly) ? wholeOnly : null;
    }
    var tail = s.slice(lastSepIdx + 1);
    var head = s.slice(0, lastSepIdx).replace(/[.,]/g, "");
    var isDecimal = tail.length >= 1 && tail.length <= 2 && /^\d+$/.test(tail);
    if (isDecimal) {
      var whole = parseInt(head || "0", 10);
      if (!isFinite(whole)) return null;
      var frac = parseInt(tail.padEnd(2, "0").slice(0, 2), 10);
      return whole + frac / 100;
    }
    var wholeStr = (head + tail).replace(/\D/g, "");
    var wholeNum = parseInt(wholeStr || "0", 10);
    return isFinite(wholeNum) ? wholeNum : null;
  }

  function parseMoney(str) {
    if (!str) return null;
    var m = MONEY_RE.exec(str);
    if (!m) return null;
    var sym = m[1] || m[4];
    var numRaw = m[2] || m[3];
    var val = normalizeAmountToken(numRaw);
    if (val === null || !isFinite(val) || val <= 0 || val >= 100000) return null;
    return { value: val, currency: sym === "$" ? "USD" : sym === "€" ? "EUR" : "GBP" };
  }

  function firstMatchingText(el, re) {
    if (!el) return null;
    var m = re.exec(textOf(el));
    return m ? m[0].trim() : null;
  }

  // Walks up from `el` a few levels looking for section/row/seat/quantity
  // labels near the price - real listing rows/cards always keep this detail
  // close to the price visually, so "close in the DOM tree" is a reasonable
  // stand-in for "close on screen" without needing real layout/CSS.
  function nearbyListingContext(el) {
    var node = el;
    var section = null, row = null, quantity = null;
    for (var depth = 0; node && depth < 4; depth++, node = node.parentElement) {
      var t = textWithGaps(node);
      if (!section) {
        // Non-greedy capture bounded by a lookahead for the NEXT field's own
        // label (row/seat/qty/quantity/a quantity+tickets phrase), a
        // currency symbol, punctuation, or end of string - stops "Section
        // 114 Row 12" at "114" instead of greedily swallowing "114 Row 12"
        // as one string. Still allows a real multi-word section name
        // through (e.g. "Grandstand Outfield 413") since the lookahead only
        // stops it at an actual next-field boundary, not at every space.
        var sm = /\bsec(?:tion)?\.?\s*[:#]?\s*([A-Za-z0-9 \-]{1,24}?)(?=\s*(?:\brow\b|\bseat\b|\bqty\b|\bquantity\b|\d{1,3}\s*(?:tickets?|seats?)\b|[$€£]|[,.]|$))/i.exec(t);
        if (sm) section = sm[1].trim();
      }
      if (!row) {
        var rm = /\brow\.?\s*[:#]?\s*([A-Za-z0-9\-]{1,8})/i.exec(t);
        if (rm) row = rm[1].trim();
      }
      if (quantity === null) {
        var qm = /(\d{1,3})\s*(?:tickets?|seats?)\b/i.exec(t) || /\bqty\.?\s*[:#]?\s*(\d{1,3})/i.exec(t);
        if (qm) {
          var qn = parseInt(qm[1], 10);
          if (isFinite(qn) && qn > 0 && qn < 1000) quantity = qn;
        }
      }
      // Stop climbing once this ancestor's own text is already large - past
      // a real card/row boundary, "nearby" stops meaning anything (would
      // start picking up unrelated listings' text on a dense results page).
      if (t.length > 600) break;
    }
    return { section: section, row: row, quantity: quantity };
  }

  // Tries each selector in order; the first one that matches ANYTHING wins
  // (never merges partial matches from several selectors - a real listing
  // grid uses one consistent markup shape, mixing selector families would
  // just as likely double-count as genuinely find more).
  function firstMatchingSelector(selectors) {
    for (var i = 0; i < selectors.length; i++) {
      try {
        var found = document.querySelectorAll(selectors[i]);
        if (found.length > 0) return { selector: selectors[i], elements: found };
      } catch (e) { /* an invalid/unsupported selector on this engine - skip it, never fail the whole read */ }
    }
    return null;
  }

  // Turns a NodeList of "this element is probably one listing" candidates
  // into {prices, listings} - every element that yields a price AND
  // correlated section/row/quantity context becomes a real `listings[]`
  // entry (high confidence); a price with no context still counts, but only
  // toward the bare `prices[]` total (see this file's own status-confidence
  // rule below main()).
  function readCandidates(elements) {
    var prices = [];
    var listings = [];
    var seen = {};
    var currency = null; // first candidate's currency wins - matches every other pass's own "if (!currency)" convention
    var n = Math.min(elements.length, MAX_CANDIDATES);
    if (n > 0) {
      // Side channel for diagnostics only (added on review) - remembers a
      // container around the actual candidates this pass looked at, so a
      // failed/partial read's DOM snapshot centers on the listings area
      // instead of always defaulting to the whole <body> (marko's spec
      // item 8 asks for a snapshot of "the relevant page section", and the
      // full body is rarely that on a real page - mostly nav/header/footer
      // markup). Never influences prices/listings themselves.
      lastCandidateContainer = elements[0].parentElement || elements[0];
    }
    for (var i = 0; i < n; i++) {
      var el = elements[i];
      var money = parseMoney(textOf(el));
      if (!money) continue;
      var ctx = nearbyListingContext(el);
      var key = money.value + "|" + money.currency + "|" + (ctx.section || "") + "|" + (ctx.row || "");
      if (seen[key]) continue; // same price+context read twice (nested selector matches) - count once
      seen[key] = true;
      prices.push(money.value);
      if (!currency) currency = money.currency;
      if (ctx.section || ctx.row) {
        listings.push({ price: money.value, currency: money.currency, section: ctx.section, row: ctx.row, quantity: ctx.quantity });
      }
    }
    return { prices: prices, listings: listings, currency: currency };
  }

  // ---------------------------------------------------------------------
  // Marketplace-specific readers
  // ---------------------------------------------------------------------

  // StubHub: prior research in this codebase (see price_checker_auto.rs's
  // own module doc comment) found no JSON-LD price data on a real StubHub
  // event page - its listing grid is rendered entirely client-side. Large
  // React apps built the way StubHub's own engineering blog has described
  // theirs commonly tag interactive rows with `data-testid` for their own
  // QA automation; that convention (not a confirmed live selector) is what
  // layers 1-2 below are grounded in. Layer 3 (added while building the
  // jsdom harness - this file's earlier comment already claimed a layer 3
  // existed here, but the code never actually had one until now) is
  // `readGeneric()`'s own JSON-LD/table/og:price/body-text-regex passes,
  // used as a genuine last resort: none of the guessed StubHub-specific
  // selectors matching (or matching only skeleton/placeholder elements with
  // no parseable price yet) doesn't mean the page has no readable price at
  // all, and marko's spec is explicit that a bare price is still better
  // than giving up (it'll come back "partial", never silently "ok").
  function readStubHub() {
    var found = firstMatchingSelector([
      '[data-testid*="listing" i]',
      '[data-testid*="ticket" i]',
      '[data-testid*="offer" i]',
      '[class*="listing" i][class*="row" i]',
      '[class*="listing" i][class*="card" i]',
      'li[role="listitem"]',
    ]);
    if (found) {
      var result = readCandidates(found.elements);
      if (result.prices.length > 0) return result;
    }
    return readGeneric();
  }

  // Vivid Seats: the ONE marketplace this codebase already confirmed (2.1.1
  // research, against a real live event page) exposes a genuine
  // Section/Row/Price/Deal-Score <table>. Kept as the primary, most-trusted
  // layer; a div/grid-card fallback is added in case that markup has since
  // changed (defense in depth - this session cannot re-confirm live either
  // way, network to vividseats.com is blocked here too). Same last-resort
  // `readGeneric()` layer as readStubHub()/readTicombo() below that too.
  function readVividSeats() {
    var listings = [];
    var prices = [];
    var currency = null;
    document.querySelectorAll("table").forEach(function (table) {
      var headerEl = table.querySelector("thead") || table;
      var headerCells = Array.prototype.map.call(headerEl.querySelectorAll("th"), function (th) { return textOf(th).toLowerCase(); });
      var headerText = textOf(headerEl).toLowerCase();
      if (headerText.indexOf("section") === -1 && headerText.indexOf("price") === -1) return;
      var sectionCol = headerCells.indexOf("section");
      var rowCol = headerCells.indexOf("row");
      var qtyCol = headerCells.findIndex(function (h) { return h.indexOf("qty") !== -1 || h.indexOf("quantity") !== -1; });
      // Prefer a header cell that actually says "price" (and not "fee"/
      // "service") - found while adversarially reviewing this file: without
      // this, a table with MORE than one money-shaped column (ticket price
      // + a service fee column, say) just took "the first cell that happens
      // to parse as money" below, which could silently pick the wrong one.
      // Falls back to that same original behavior when no such header
      // exists, rather than refusing to read the table at all.
      var priceCol = headerCells.findIndex(function (h) { return h.indexOf("price") !== -1 && h.indexOf("fee") === -1 && h.indexOf("service") === -1; });
      table.querySelectorAll("tbody tr, tr").forEach(function (tr) {
        var cells = tr.querySelectorAll("td");
        if (cells.length < 3) return;
        var cellText = function (i) { return i >= 0 && i < cells.length ? textOf(cells[i]) : null; };
        var money = priceCol >= 0 && priceCol < cells.length ? parseMoney(textOf(cells[priceCol])) : null;
        if (!money) {
          for (var i = 0; i < cells.length; i++) {
            money = parseMoney(textOf(cells[i]));
            if (money) break;
          }
        }
        if (!money) return;
        prices.push(money.value);
        if (!currency) currency = money.currency;
        lastCandidateContainer = table; // diagnostics-only, see readCandidates's own comment
        var qtyRaw = qtyCol >= 0 ? cellText(qtyCol) : null;
        var qtyNum = qtyRaw ? parseInt(qtyRaw.replace(/[^\d]/g, ""), 10) : NaN;
        listings.push({
          price: money.value, currency: money.currency,
          section: sectionCol >= 0 ? cellText(sectionCol) : cellText(0),
          row: rowCol >= 0 ? cellText(rowCol) : cellText(1),
          quantity: isFinite(qtyNum) ? qtyNum : null,
        });
      });
    });
    if (listings.length > 0) return { prices: prices, listings: listings, currency: currency };

    // Fallback: a card/grid layout instead of a <table>.
    var found = firstMatchingSelector([
      '[data-testid*="listing" i]',
      '[data-testid*="ticket" i]',
      '[class*="listing" i][class*="card" i]',
      'li[role="listitem"]',
    ]);
    if (found) {
      var cardResult = readCandidates(found.elements);
      if (cardResult.prices.length > 0) return cardResult;
    }
    return readGeneric();
  }

  // Ticombo: prior research (same doc comment) found `og:price:amount` /
  // `og:price:currency` meta tags in the markup PATTERN, but no listed
  // (non-expired) event page available at the time to confirm a populated
  // value. Kept as layer 1 (cheap, structured, trustworthy when present);
  // layer 2 is a card-pattern guess for its actual listing grid, same
  // reasoning as StubHub's layers above. Same last-resort `readGeneric()`
  // layer as readStubHub()/readVividSeats() too.
  function readTicombo() {
    var amountMeta = document.querySelector('meta[property="og:price:amount"]');
    var amount = amountMeta ? parseFloat(amountMeta.getAttribute("content")) : NaN;
    if (isFinite(amount) && amount > 0) {
      var curMeta = document.querySelector('meta[property="og:price:currency"]');
      // A single aggregate meta tag - by construction never has section/row
      // context, so this only ever feeds `prices`, never `listings`
      // directly (see main()'s own confidence rule).
      return { prices: [amount], listings: [], currency: (curMeta && curMeta.getAttribute("content")) || null };
    }
    var found = firstMatchingSelector([
      '[data-testid*="price" i]',
      '[data-testid*="listing" i]',
      '[class*="price-card" i]',
      '[class*="listing" i][class*="item" i]',
    ]);
    if (found) {
      var cardResult = readCandidates(found.elements);
      if (cardResult.prices.length > 0) return cardResult;
    }
    return readGeneric();
  }

  // Generic (any 4th/5th marketplace marko adds himself, including
  // Viagogo - not in his list of three needing a dedicated reader): the
  // same marketplace-agnostic multi-pass logic this file has used since
  // 2.1.1, unchanged in substance. This is deliberate, not a fallback born
  // of laziness - see price_checker_auto.rs's own "Extraction strategy"
  // doc comment for why hard-coding by marketplace NAME would quietly
  // break the moment marko renames or adds one.
  function readGeneric() {
    var prices = [];
    var currency = null;
    var listings = [];

    document.querySelectorAll('script[type="application/ld+json"]').forEach(function (s) {
      try {
        var data = JSON.parse(s.textContent);
        var items = Array.isArray(data) ? data : [data];
        items.forEach(collectOffers);
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

    if (prices.length === 0) {
      document.querySelectorAll("table").forEach(function (table) {
        var headerEl = table.querySelector("thead") || table;
        var headerCells = Array.prototype.map.call(headerEl.querySelectorAll("th"), function (th) { return textOf(th).toLowerCase(); });
        var headerText = textOf(headerEl).toLowerCase();
        if (headerText.indexOf("section") === -1 && headerText.indexOf("price") === -1) return;
        var sectionCol = headerCells.indexOf("section");
        var rowCol = headerCells.indexOf("row");
        var qtyCol = headerCells.findIndex(function (h) { return h.indexOf("qty") !== -1 || h.indexOf("quantity") !== -1; });
        // Same priceCol preference as readVividSeats's own table pass above
        // (found while adversarially reviewing this file) - see that
        // pass's own comment.
        var priceCol = headerCells.findIndex(function (h) { return h.indexOf("price") !== -1 && h.indexOf("fee") === -1 && h.indexOf("service") === -1; });
        table.querySelectorAll("tbody tr, tr").forEach(function (row) {
          var cells = row.querySelectorAll("td");
          if (cells.length < 3) return;
          var cellText = function (i) { return i >= 0 && i < cells.length ? textOf(cells[i]) : null; };
          var money = priceCol >= 0 && priceCol < cells.length ? parseMoney(textOf(cells[priceCol])) : null;
          if (!money) {
            for (var i = 0; i < cells.length; i++) {
              money = parseMoney(textOf(cells[i]));
              if (money) break;
            }
          }
          if (!money) return;
          prices.push(money.value);
          if (!currency) currency = money.currency;
          lastCandidateContainer = table; // diagnostics-only, see readCandidates's own comment
          var qtyRaw = qtyCol >= 0 ? cellText(qtyCol) : null;
          var qtyNum = qtyRaw ? parseInt(qtyRaw.replace(/[^\d]/g, ""), 10) : NaN;
          listings.push({
            price: money.value, currency: money.currency,
            section: sectionCol >= 0 ? cellText(sectionCol) : cellText(0),
            row: rowCol >= 0 ? cellText(rowCol) : cellText(1),
            quantity: isFinite(qtyNum) ? qtyNum : null,
          });
        });
      });
    }

    if (prices.length === 0) {
      var amountMeta = document.querySelector('meta[property="og:price:amount"]');
      var amount = amountMeta ? parseFloat(amountMeta.getAttribute("content")) : NaN;
      if (isFinite(amount) && amount > 0) {
        prices.push(amount);
        var curMeta = document.querySelector('meta[property="og:price:currency"]');
        currency = (curMeta && curMeta.getAttribute("content")) || currency;
      }
    }

    if (prices.length === 0) {
      var bodyText = document.body ? (document.body.innerText || document.body.textContent || "") : "";
      var CONFIDENT = /([€$£])\s?(\d[\d.,]*\d|\d)|(\d[\d.,]*\d|\d)\s?([€$£])(?!\d)/g;
      var genericFound = [];
      var gm;
      while ((gm = CONFIDENT.exec(bodyText)) !== null) {
        var sym = gm[1] || gm[4];
        var numRaw = gm[2] || gm[3];
        // Same locale-aware parse as parseMoney() above (not the old
        // comma-strip) - see normalizeAmountToken's own comment for why.
        var val = normalizeAmountToken(numRaw);
        if (val !== null && isFinite(val) && val > 0 && val < 100000) genericFound.push({ val: val, sym: sym });
      }
      if (genericFound.length >= 2) {
        genericFound.forEach(function (f) {
          prices.push(f.val);
          if (!currency) currency = f.sym === "$" ? "USD" : f.sym === "€" ? "EUR" : "GBP";
        });
      }
    }

    return { prices: prices, listings: listings, currency: currency };
  }

  function hostFamily() {
    var h = (location.hostname || "").toLowerCase();
    if (h.indexOf("stubhub") !== -1) return "stubhub";
    if (h.indexOf("vividseats") !== -1) return "vividseats";
    if (h.indexOf("ticombo") !== -1) return "ticombo";
    return "generic";
  }

  // ---------------------------------------------------------------------
  // Diagnostics (marko's spec section 7 - "Aktuálna hláška je príliš
  // slabá"). Every count below is cheap (a single querySelectorAll length
  // or a regex match count) so gathering it every attempt is not a
  // performance concern. Never includes cookies/auth tokens/form values -
  // domSnapshot strips <script>/<style> and any input value/token/auth/
  // session-looking attribute before capping its length.
  // ---------------------------------------------------------------------

  function countElementsWithTextMatching(re) {
    var all = document.querySelectorAll("body, body *");
    var n = 0;
    var cap = Math.min(all.length, 5000); // hard cap - never walk an enormous page fully just for a diagnostic count
    for (var i = 0; i < cap; i++) {
      if (re.test(all[i].textContent || "")) n++;
    }
    return n;
  }

  // Found under-scoped while adversarially reviewing this file: the
  // original attribute-stripping pass only matched by attribute NAME
  // (value/data-*token/auth/session/secret*), so it would miss e.g.
  // `data-apikey`, `data-jwt`, or a token sitting in a completely
  // unrelated attribute (a `href` query string, say). This second pass
  // catches any attribute VALUE that looks like an opaque token
  // regardless of what the attribute is called. Deliberately conservative
  // (won't touch a normal sentence or a short id like "section-114") but
  // errs toward stripping a plausible-looking token marko never asked to
  // have kept.
  //
  // Broadened on a second review pass (the first version only matched a
  // value made ENTIRELY of `[A-Za-z0-9_.-]`, which excludes `+`, `/`, `=` -
  // the exact extra characters a STANDARD, non-URL-safe base64 secret uses,
  // so a real API key/session token in that shape evaded it completely) -
  // extracts each quoted attribute value and judges it on its own with
  // `looksLikeToken`, rather than one dense inline regex.
  function looksLikeToken(value) {
    if (!value || value.length < 16 || /\s/.test(value)) return false;
    if (/^\d{16,}$/.test(value)) return true; // long all-digit run (card/session/order-id shaped)
    if (/^[A-Za-z]{24,}$/.test(value)) return true; // long all-letter run
    // Mixed opaque token, base64-alphabet-inclusive (letters, digits, and
    // _/+=), needs at least one letter AND one digit so a plain
    // human-readable path/slug isn't caught here too. Deliberately
    // EXCLUDES "-" and "." from this branch specifically (unlike the JWT/
    // Bearer/labeled-token patterns above, which do allow them) - a real
    // ticket marketplace's own markup is full of legitimate hyphenated
    // slugs that mix words and numbers ("grandstand-outfield-413",
    // "section-114-row-12"), and those must never be redacted just for
    // being long and containing a digit. A hyphen/dot-free run this long
    // with no natural word structure is a much safer, more specific signal
    // that it's actually opaque.
    return value.length >= 24 && /^[A-Za-z0-9_/+=]+$/.test(value) && /[A-Za-z]/.test(value) && /\d/.test(value);
  }

  function stripSuspiciousAttributeValues(html) {
    return html.replace(/=("|')([^"']*)\1/g, function (whole, quote, value) {
      return looksLikeToken(value) ? "=" + quote + "[redacted]" + quote : whole;
    });
  }

  function safeDomSnapshot(container) {
    var html = (container || document.body || {}).outerHTML || "";
    html = html.replace(/<script[\s\S]*?<\/script>/gi, "").replace(/<style[\s\S]*?<\/style>/gi, "");
    html = html.replace(/\s(value|data-[\w-]*(?:token|auth|session|secret)[\w-]*)\s*=\s*("[^"]*"|'[^']*')/gi, "");
    html = stripSuspiciousAttributeValues(html);
    return html.slice(0, DOM_SNAPSHOT_MAX_CHARS);
  }

  // Never let visible page text carry an opaque token-shaped string into a
  // message marko might copy/paste to ask for help (exactly how this whole
  // 2.1.8 rewrite started - he pasted a diagnostic message back) or into
  // the (pre-existing, 2.1.6) AI fallback's request body. Page text is
  // very unlikely to literally contain a cookie (those live in
  // document.cookie/HTTP headers, never read here - see this file's own
  // "no cookies" guarantee), but a broken/debug page could still render a
  // session id, API key, or bearer token as visible text, so this is
  // cheap, conservative insurance rather than a response to a confirmed
  // real leak.
  function scrubSensitiveText(str) {
    if (!str) return str;
    var s = str.replace(/\bBearer\s+[A-Za-z0-9_\-.+/=]{10,}/gi, "Bearer [redacted]");
    s = s.replace(/\beyJ[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}/g, "[redacted-jwt]");
    s = s.replace(/\b(token|session ?id|auth|api[_-]?key|secret)\b\s*[:=]\s*[A-Za-z0-9_\-.+/=]{8,}/gi, "$1 [redacted]");
    // Broadened on a second review pass, same reasoning as
    // stripSuspiciousAttributeValues above: also catch a long ALL-digit run
    // (16+, card/session/order-id shaped) or ALL-letter run (24+), and
    // widen the mixed-run alphabet to include +/= so a standard
    // (non-URL-safe) base64 secret doesn't slip through by using a
    // character the old class didn't allow. Ordinary prose essentially
    // never hits any of these three. The mixed-run pattern deliberately
    // excludes "-" - see looksLikeToken's own comment for why a hyphenated
    // slug/section-id must never be swept up here.
    s = s.replace(/\b\d{16,}\b/g, "[redacted]");
    s = s.replace(/\b[A-Za-z]{24,}\b/g, "[redacted]");
    s = s.replace(/(?=[A-Za-z0-9_+/=]*[A-Za-z])(?=[A-Za-z0-9_+/=]*\d)[A-Za-z0-9_+/=]{24,}/g, "[redacted]");
    return s;
  }

  function collectDiagnostics(family, attempt, candidateCount, bestContainer) {
    var bodyText = document.body ? (document.body.innerText || document.body.textContent || "") : "";
    return {
      marketplaceReader: family,
      attempt: attempt,
      pageTitle: document.title || null,
      finalUrl: location.href,
      domLength: (document.documentElement ? document.documentElement.outerHTML.length : 0),
      visibleTextLength: bodyText.length,
      tableCount: document.querySelectorAll("table").length,
      linkCount: document.querySelectorAll("a").length,
      buttonCount: document.querySelectorAll("button").length,
      currencySymbolElementCount: countElementsWithTextMatching(/[€$£]/),
      priceTextElementCount: countElementsWithTextMatching(/price/i),
      sectionTextElementCount: countElementsWithTextMatching(/\bsection\b/i),
      rowTextElementCount: countElementsWithTextMatching(/\brow\b/i),
      candidateListingElementCount: candidateCount,
      textSample: scrubSensitiveText(bodyText.slice(0, 600)),
      // 2.1.6: longer slice for the AI-assisted fallback - unchanged shape/
      // size, still only ever read when everything else here found nothing.
      // Scrubbed same as textSample above (added on review) since this one
      // actually leaves the machine, sent to the Anthropic API.
      aiText: scrubSensitiveText(bodyText.slice(0, 8000)),
      domSnapshot: safeDomSnapshot(bestContainer),
    };
  }

  // ---------------------------------------------------------------------
  // main
  // ---------------------------------------------------------------------

  var titleLower = (document.title || "").toLowerCase();
  var blocked = ["just a moment", "attention required", "access denied", "pardon our interruption", "are you a human", "verify you are human"].some(
    function (s) { return titleLower.indexOf(s) !== -1; }
  );

  // Tracks how many extraction attempts have run on THIS page load, purely
  // for diagnostics (poll_then_extract, Rust side, is the actual source of
  // truth for retry control/budget - this counter never influences what
  // gets extracted, only what gets reported alongside it).
  window.__tiqrExtractAttempt = (window.__tiqrExtractAttempt || 0) + 1;

  var family = blocked ? "blocked" : hostFamily();
  var result = { prices: [], listings: [], currency: null };
  // Top-level guard (added on review): marko's spec is explicit every
  // attempt must be diagnosable. Without this, a reader throwing on some
  // as-yet-unseen real page shape would make the WHOLE eval fail
  // (EvalOutcome::Failed on the Rust side), which carries NO diagnostics
  // at all - the exact opposite of diagnosable. An error here degrades to
  // "found nothing this attempt" (safe - the retry loop just tries again)
  // with the error folded into the text sample so it's still visible if
  // marko copies the message back, instead of an opaque eval failure.
  var readerError = null;
  if (!blocked) {
    try {
      if (family === "stubhub") result = readStubHub();
      else if (family === "vividseats") result = readVividSeats();
      else if (family === "ticombo") result = readTicombo();
      else result = readGeneric();
    } catch (e) {
      readerError = String((e && e.message) || e);
      result = { prices: [], listings: [], currency: null };
    }
  }

  var diagnostics;
  try {
    diagnostics = collectDiagnostics(family, window.__tiqrExtractAttempt, result.listings.length || result.prices.length, lastCandidateContainer);
  } catch (e2) {
    // Even diagnostics collection itself failing must not crash the whole
    // eval - fall back to the handful of fields that don't touch the DOM.
    diagnostics = {
      marketplaceReader: family,
      attempt: window.__tiqrExtractAttempt,
      pageTitle: document.title || null,
      finalUrl: location.href,
      textSample: "[diagnostics collection failed: " + String((e2 && e2.message) || e2) + "]",
    };
  }
  if (readerError) {
    diagnostics.textSample = ("[reader error: " + readerError + "] " + (diagnostics.textSample || "")).slice(0, 600);
  }

  return JSON.stringify({
    prices: result.prices,
    currency: result.currency,
    blocked: blocked,
    listings: result.listings,
    diagnostics: diagnostics,
    title: document.title,
  });
})();
