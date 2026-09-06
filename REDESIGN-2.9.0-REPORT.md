# TIQR Manager 2.9.0 — Price Checker: Accuracy + Feature Upgrade

Toto je report k tvojmu zadaniu. Držal som sa poradia, ktoré si určil:
**najprv diagnostika, potom najmenšia oprava, potom basic improvements.**
Nič som neprepisoval od nuly.

---

## ⚠️ Čo som NEMOHOL overiť (čítaj toto ako prvé)

Dve rôzne veci, obe dôležité:

1. **Build a testy som nespustil.** Na Macu nie je Node ani Rust. Pribudlo
   15 nových Rust testov (43 v module), **ani jeden nebežal**.
2. **DOM opravy som nemohol overiť na živom marketplace.** Tento sandbox
   nemá prístup na stubhub/vividseats/ticombo — to platí pre tento skript už
   od 2.1.9. Navyše **injektovaný JS sa tu nedá ani spustiť** (nie je Node),
   takže parser nemá test vôbec.

**Príčiny, ktoré popisujem nižšie, sú ale dokázateľné priamo z kódu** — nie
sú to odhady. Každú viem ukázať na konkrétnom riadku. Čo je odhad, to
označujem.

Pred publikovaním:
```
npm install && npx tsc -b && npm run build && cargo check --lib && cargo test --lib
```

---

## 1. Presná príčina scan bugov

Našiel som **deväť** konkrétnych defektov. Toto je ten hlavný:

### 🔴 B1 — Cena sa čítala z iného elementu, než v ktorom sa našla

`price_checker_scan.js`, `readGenericVisibleText`:

```js
var money = parseMoney(text);           // text = node.nodeValue (TEXT NODE)
if (!money) continue;
var c = candidateFrom(parent, marketplace);   // ← znova parsuje CELÝ parent
```

`candidateFrom` potom robilo `parseMoney(accessibleText(parent) || parent.textContent)`
a `parseMoney` vracia **prvý** match.

**Takže uložená cena nikdy nebola garantovane tá, ktorá sa našla.**

Dôsledky, presne tie, ktoré si hlásil:

| Markup | Našlo sa | Uložilo sa |
|---|---|---|
| `<s>$200</s> <b>$120</b>` | $120 | **$200** (prečiarknutá stará cena) |
| `aria-label="Total incl. fees $340"` | cena riadku | **$340** (total s fees) |
| `Fees $12` pred cenou | cena | **$12** |

**Toto je root cause.** Ostatné defekty ho zhoršujú.

### Ďalších osem

| # | Defekt | Kde |
|---|---|---|
| B2 | **Žiadna kontrola prečiarknutia.** Nič nekontrolovalo `<s>/<del>`, `line-through`, ani `was-price` triedy. `isVisible` ich explicitne púšťalo ďalej. | `scan.js` |
| B3 | **Žiadne odmietnutie fees/total/header/footer.** Akýkoľvek money-shaped text bol platná cena. | `scan.js` |
| B4 | **Únik metadát.** `findListingContainer` padalo na "3 predkov hore" a `nearbyListingContext` potom regexovalo **celý ten subtree** → section/row/qty/tier sa brali zo **susedného listingu** alebo z page chrome. | `scan.js` |
| B5 | **Dedupe malo cenu v kľúči.** `fingerprint_for` = `marketplace\|price\|currency\|section\|row\|qty\|id`. Ten istý listing prečítaný po scrolle sa počítal **dvakrát**, len čo sa cena prečítala inak — čo B1 spôsoboval bežne. | `scanner.rs` |
| B6 | **Dedupe zároveň mazalo reálne listings.** `tier` v kľúči **nebol** → dva listingy líšiace sa len tierom sa zlúčili do jedného. | `scanner.rs` |
| B7 | **In-scan dedupe podľa container node.** S B4 fallbackom mohlo viac rôznych listingov zdieľať jeden fallback ancestor → všetky okrem prvého zmizli. | `scan.js` |
| B8 | **Žiadne accounting.** Existovalo iba `added_this_scan`. Žiadne Found/Accepted/Skipped/Duplicates. | `scanner.rs` |
| B9 | **Miešanie mien.** `compute_scan_stats` počítalo min/median/avg/max cez **všetky** listingy bez ohľadu na menu a výsledok označilo **prvou** nájdenou menou. | `scanner.rs` |

**B9 je obzvlášť nepríjemný**, lebo `price_checker_analysis.rs` menu vždy
delil správne (`partition_by_currency`, "never blended"). Takže **headline
štatistika hore a Market Analysis hneď pod ňou si mohli protirečiť na tých
istých dátach.**

---

## 2. Ktoré readers/parsers boli opravené

- **`price_checker_scan.js`** — B1, B2, B3, B4, B7 + skip accounting
- **`price_checker_scanner.rs`** — B5, B6, B8, B9

**Marketplace readers som nepridával ani nemenil.** `readStubHub` /
`readVividSeats` / `readTicombo` sú naďalej tenké wrappery nad
`scanWithSelectors` — opravy sú v zdieľanej vrstve, kde boli aj chyby.
Žiadny nový marketplace, žiadny obrovský univerzálny parser.

---

## 3. Ako sa zlepšila accuracy

**Cena sa berie z toho, čo sa našlo:**
```js
candidateFrom(parent, marketplace, money, text)   // matched money + matched text
```

**Tri nové rejection pravidlá** (predtým neexistovalo ani jedno):

1. **`isStruckThrough(el)`** — `<s>/<del>/<strike>`, trieda obsahujúca
   `strike|line-through|was-price|old-price|original-price|crossed`, alebo
   computed `text-decoration: line-through`. Kontroluje element + 4 predkov.
2. **`NON_LISTING_PRICE_RE`** — total, subtotal, fees, service charge,
   booking fee, delivery, shipping, tax, VAT, balance, savings, discount,
   was, before, previously, original, RRP, face value.
3. **`inExcludedRegion(el)`** — `<header>/<footer>/<nav>/<aside>`,
   `role=banner|contentinfo|navigation|dialog`, alebo trieda
   `cart|basket|checkout|summary|header|footer|nav|cookie|consent|modal|drawer`.

Všetky tri sú **zámerne konzervatívne** — odmietajú iba keď text sám hovorí,
čo to číslo je. Nikdy neodmietnu holé číslo.

**A čo je dôležité: každé odmietnutie sa počíta a zobrazí.** Ak by bolo
niektoré pravidlo príliš agresívne, uvidíš to ako číslo v Skipped, nie ako
chýbajúce dáta.

**Metadata scoping:** `findListingContainer` teraz vracia
`{el, confident}`. Keď `confident: false`, metadata sa čítajú z tesného
rozsahu (parent ceny) a listing sa označí `incomplete` — nikdy sa
neprezentuje ako čistý read.

**Nič sa nevymýšľa.** Keď scanner hodnotu nevie, je `null`. Žiadny fallback,
žiadny odhad.

---

## 4. Dedupe mechanizmus

| Situácia | Kľúč |
|---|---|
| Marketplace dá **listing ID** | `marketplace\|id:<id>` — **iba ID** |
| Bez ID | `marketplace\|price\|currency\|section\|row\|qty\|tier` |

**S ID je identita celá.** ID je identita z definície — nič iné sa nemusí
zhodovať. To rieši scroll/rerender duplicity definitívne.

**Bez ID cena v kľúči zostáva — zámerne.** Bez stabilnej identity sa nedá
odlíšiť „ten istý listing, znova prečítaný" od „druhý listing za inú cenu v
tom istom rade". A z týchto dvoch chýb je **ponechaný duplikát opraviteľný,
zmazaný reálny listing nie.** Presne to, čo si žiadal v Časti C.

`tier` pribudol do fallback kľúča (oprava B6). Textové polia sa trimujú a
lowercasujú, takže whitespace/case rozdiel medzi dvoma rendermi nie je nový
listing.

---

## 5. Scan summary

```
Last scan   Found 184   Accepted 161   Skipped 14   Duplicates 9
            crossed-out / was price: 8   fees, total or other non-listing price: 4   header, footer, cart or nav: 2
```

`found` = všetko money-shaped, čo obe vrstvy rozpoznali **pred** akýmkoľvek
rejection pravidlom. `accepted + skipped` vždy sedí na `found` — preto je to
poctivé číslo, nie dekorácia.

`Duplicates` = tie, čo skript zlúčil v rámci jedného scanu **+** tie, ktoré
odmietol fingerprint naprieč session.

---

## 6. Nové Price Checker features

- **Scan summary** (Časť D) — vyššie
- **Filtre** (Časť L) — marketplace, tier, currency, min/max cena,
  complete/incomplete. Šesť obyčajných ovládacích prvkov, žiadny filter
  builder. Možnosti sa generujú z toho, čo scan reálne vrátil — nikdy fixný
  zoznam, takže nemôže existovať prázdna voľba.
- **Export CSV** (Časť K) — stĺpce: `event_id, marketplace, listing_id, url,
  price, currency, tier, section, row, quantity, complete`. Používa **ten
  istý** mechanizmus ako každý iný export v appke: `plugin-dialog` `save()` +
  Rust `csv::Writer`. Prázdna hodnota = nie je k dispozícii, nikdy „N/A".
- **`incomplete` označenie** — hviezdička v tabuľke s vysvetlením v tooltipe.
- **Currency warning** — keď session drží viac mien, pod štatistikou je
  explicitná veta, koľko listingov je v inej mene a že sa **nemiešajú**.

---

## 7. Market Analysis zmeny

**`price_checker_analysis.rs` som nemenil vôbec** — bol už správny
(`partition_by_currency`, tier breakdown, comparables). Total listings, min,
median, average, max, currency-aware handling aj tier breakdown tam už boli.

Opravil som **konzistenciu smerom k nemu**: `compute_scan_stats` (headline
štatistika scannera) teraz počíta v **najväčšej jednomenovej skupine** a
vracia, koľko listingov vynechala. Listingy bez meny sú vylúčené, nie
priradené k väčšine. Keď nemá menu nič, štatistika je `None` — nikdy číslo
bez jednotky.

Breakdown podľa marketplace je dostupný cez nový filter (jeden scan = jeden
marketplace window, takže per-marketplace čísla dáva filter priamo).

---

## 8. Tier / Level pravidlo

**Dodržané bez výnimky.**

- Tier/Level = market grouping (breakdown, filter, dedupe key)
- Section/Row/Seat = **iba metadata** (stĺpce v tabuľke a v CSV)
- **Žiadny pricing podľa section/row/seat**
- **Žiadne automatic price suggestions, žiadny repricing**

Tvoj príklad — Tier 100 so Section A €100 a Section B €200 — nikde
nevyprodukuje „fair price €150". Systém takú hodnotu nepočíta.

---

## 9. History / export / filter changes

- **History**: infraštruktúra už fungovala a **nemenil som ju**. Manuálny scan
  ide naďalej cez ten istý review-then-save krok (`Save to history`) a nesie
  timestamp, marketplace, listing count, market stats aj tier breakdown.
  **Žiadne continuous monitoring, žiadne background/scheduled snapshots** —
  history vzniká iba z manuálneho scanu, presne ako doteraz.
- **Export**: nový, popísaný vyššie.
- **Filtre**: nové, popísané vyššie.

---

## 10. Zmenené súbory

**Backend (5):**
- `commands/price_checker_scan.js` — B1, B2, B3, B4, B7 + skip accounting (637 → 790 riadkov)
- `commands/price_checker_scanner.rs` — B5, B6, B8, B9 + export command + 15 testov
- `models.rs` — `NormalizedListing.incomplete`, 7 nových polí na `ScanResultPayload`
- `db.rs` — `ScannerSession.url`
- `lib.rs` — registrácia `export_scan_results_csv`

**Frontend (3):**
- `pages/PriceChecker.tsx` — `ScanResultsPanel` (summary + filtre + export)
- `lib/types.ts` — mirror nových polí
- `lib/api.ts` — `exportScanResultsCsv`

**`price_checker_analysis.rs`**: iba `incomplete: false` v test helperi.

**Verzia (7 súborov, 9 výskytov)** + **dokumentácia (4)**.

---

## 11. DB / migration changes

**Žiadne.** Žiadna migrácia (ďalšia nová je stále **027**), žiadna tabuľka,
žiadny stĺpec, žiadny index, žiadna nová dependency.

---

## 12. Test results

### 15 nových Rust testov

| Tvoja požiadavka | Test |
|---|---|
| duplicate listing | `a_real_listing_id_is_the_whole_identity_even_when_the_price_changed` |
| scroll/rerender duplicate | `re_scanning_the_same_page_adds_nothing_and_counts_every_repeat_as_a_duplicate` |
| listing boundaries | `two_listings_differing_only_by_tier_are_not_merged` |
| — | `without_an_id_a_different_price_is_still_treated_as_a_different_listing` |
| — | `listing_id_identity_is_still_scoped_to_its_marketplace` |
| — | `whitespace_and_case_differences_are_not_a_different_listing` |
| mixed currencies | `stats_never_blend_two_currencies_into_one_number` |
| invalid currency | `listings_with_no_currency_are_excluded_from_stats_rather_than_assumed` |
| — | `stats_are_none_when_nothing_has_a_currency_at_all` |
| — | `the_largest_currency_group_wins_and_ties_are_deterministic` |
| scan summary counts | `scan_summary_counts_reach_the_payload_and_duplicates_add_up` |
| incomplete listings | `the_incomplete_flag_survives_the_trip_from_the_page_reader` |
| — | `a_payload_from_before_the_summary_existed_still_deserializes` |

### Čo NIE JE otestované

**Price extraction, current-vs-crossed-out, currency extraction,
section/row/tier extraction a listing boundaries na úrovni DOM** — to všetko
žije v `price_checker_scan.js`, ktorý sa tu **nedá spustiť** (nie je Node).
Nemá test. Nechcem tvrdiť opak.

### Statická kontrola

| | |
|---|---|
| Zátvorky vo všetkých 8 zmenených súboroch | 0 / 0 / 0 |
| Všetkých 43 `#[test]` naviazaných na `fn` | ✓ |
| Importy v `PriceChecker.tsx` | všetky sa rozlišujú |
| Každý `NormalizedListing` / `ScannerSession` literál má nové polia | ✓ |
| Všetky `insert_new_session` call sites | ✓ (9) |
| `$CommitMsg` bez úvodzoviek | ✓ (guard z 2.7.0) |

**Jednu reálnu chybu som pritom našiel a opravil**: `decimalStringToCents("")`
vracia **0**, nie `null` — prázdne pole „Max price" by odfiltrovalo úplne
všetko. Prázdny vstup teraz explicitne znamená „bez limitu".

---

## 13. Čo nebolo možné live overiť

- **Celý DOM parsing.** Marketplaces nie sú z tohto sandboxu dostupné (platí
  od 2.1.9) a JS sa tu nedá spustiť.
- **Či sú selectory v `LISTING_PRICE_SELECTORS` stále aktuálne.** Nemenil som
  ich — sú to loose attribute-contains selectory a layer 3 (generic text) beží
  vždy tak či tak.
- **Reálne pomery Found/Accepted/Skipped.** Aritmetika je otestovaná, ale
  aké čísla uvidíš na skutočnej stránke, neviem.

**Čo urob ty:** spusti scan na reálnej stránke a pozri sa na summary. Ak je
Skipped podozrivo vysoké, pošli mi rozpis dôvodov — z toho presne uvidím,
ktoré pravidlo je príliš prísne. Práve preto som každé odmietnutie počítal.

---

## 14. Limity

- **Bez listing ID zostáva cena v dedupe kľúči.** Ak ten istý listing reálne
  zmení cenu medzi dvoma scanmi a stránka nedáva ID, započíta sa dvakrát.
  Vedomý kompromis — opak by mazal reálne listingy.
- **`incomplete` je jeden boolean**, nie confidence per pole. Scanner
  úprimne nevie ohodnotiť každé pole zvlášť a predstierať to by bolo
  vymýšľanie si istoty.
- **Rejection pravidlá sú anglické keywordy.** Na neanglickej verzii stránky
  „total"/„fees" nezachytia. Rozšíriť sa dá, ale bez reálnej stránky by som
  hádal, ktoré jazyky.
- **Layer 4 (screenshot/OCR) naďalej neexistuje** — nezmenené od 2.1.9.
- **Market Analysis som nerozširoval** nad rámec Časti G — už mal všetko, čo
  si vymenoval.

---

## Čo sa zámerne NEZMENILO

refund/resell · `batch_id` · money/integer cents · Orders · Tickets · Sales ·
Listings · Finance · Fulfillment · Attention · Calendar · Google Sheets ·
manuálny Visible Scanner workflow · Your Tickets comparison · history
infraštruktúra · `price_checker_analysis.rs` · marketplace readers.

Žiadny background monitor, žiadny scheduled scan, žiadny polling, žiadny
CAPTCHA bypass, žiadny nový marketplace, žiadne AI volanie.
