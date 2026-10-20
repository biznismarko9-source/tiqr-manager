# TIQR Manager 2.10.0 — Price Checker: Event Overview

Report k tvojmu zadaniu. Dropdown je preč — Price Checker sa teraz otvára
zoznamom všetkých relevantných eventov.

---

## ⚠️ Build som nespustil

Na Macu stále nie je Node ani Rust. `cargo test --lib`, `cargo check --lib`,
`npx tsc -b`, `npm run build` — ani jedno. Pribudlo 8 nových Rust testov,
nebežali.

Špecificky som ale skontroloval **presne tú triedu chyby, ktorá zhodila
2.9.0** (miešané typy v poli) — v novom kóde žiadne pole ani `vec![]` nie je.

```
npm install && npx tsc -b && npm run build && cargo check --lib && cargo test --lib
```

---

## 1. Nový event overview

Po otvorení Price Checkera vidíš **všetky upcoming eventy naraz**, každý ako
vlastná hustá karta:

```
┌─────────────────────────────────────────────────┐
│ ☐ Coldplay — Munich                    14.09.26 │
│   Munich · Allianz Arena                        │
│                                                 │
│   Viagogo       Linked        2h ago · 184      │
│   Vivid Seats   Linked        2h ago · 96       │
│   Ticombo       No link       Not scanned       │
│   ─────────────────────────────────────────     │
│   184 listings · 2h ago              Open →     │
└─────────────────────────────────────────────────┘
```

Na širokej obrazovke dve karty vedľa seba. Žiadne obrovské karty cez pol
obrazovky.

**Ktoré eventy:** iba `status = 'upcoming'` — presne to isté pravidlo, ktoré
PriceChecker.tsx aplikoval client-side už od 2.2.2. Iba som ho presunul do
SQL, keďže stránka teraz eventy listuje, nie filtruje dropdown.

---

## 2. Multi-select

- checkbox pri každej karte
- **Select all** — zámerne obmedzený na **práve viditeľné** riadky. Keď máš
  zapnutý filter alebo search, „vybrať všetko" nesmie potichu označiť aj to,
  čo nevidíš.
- **Clear selection**
- keď je niečo označené, hore sa objaví pruh: **Selected: 3 events**
- označená karta má brand ring

---

## 3. Status info pri každom evente

Všetko z reálnych uložených riadkov:

| Údaj | Zdroj |
|---|---|
| Event name, date, venue, city | stĺpce na `events` |
| Linked / No link | existencia riadku v `event_marketplace_links` |
| Last scan (relatívny čas) | `price_checks.checked_at` |
| Listing count | `price_checks.listing_count` z toho istého najnovšieho checku |
| Event-level „184 listings · 2h ago" | najnovší check naprieč všetkými marketplace |
| „Not scanned yet" | keď event nemá žiadny check |

Relatívny čas („2h ago") má presný timestamp v tooltipe.

**Nikdy nula namiesto „nič".** Keď event nebol nikdy skenovaný, je tam
`Not scanned yet` — nie `0 listings`, čo by znamenalo „skenované, nič sa
nenašlo".

---

## 4. Marketplace link status

Zobrazujú sa iba existujúce marketplace — **Viagogo, Vivid Seats, Ticombo**.

Pravidlo, ktoré marketplace sa na evente objavia, je **to isté**, aké už
používa `get_price_checker_summary_impl`: každý aktívny vždy, plus akýkoľvek
retired, ku ktorému **tento konkrétny event** reálne má link alebo check.
Takže retirovaný marketplace (Seatriks, StubHub) sa naďalej ukáže tam, kde má
skutočnú históriu, a nikdy sa neponúkne na čistom evente.

Tri stavy: **No link** · **Linked, Not scanned** · **Linked + last scan**.

### ⚠️ Prečo tam NIE JE „SCAN FAILED"

Žiadal si aj tento stav. **Nemá zdroj dát.**

Overil som to priamo v schéme: `price_checks` má stĺpce
`id, event_id, marketplace_id, lowest/average/highest, listing_count,
currency, checked_at, is_demo` — **žiadny status**. A riadok sa tam dostane
iba cez explicitný review-then-save krok, takže **z konštrukcie je každý
uložený check úspešný**. Chybové stavy scannera (`error`, `blocked`,
`unable_to_read`) žijú iba v pamäti počas života jedného scanner okna a pri
zavretí zmiznú.

(Jediný `scan_status` v celej schéme je v migrácii 026 — to sú osirelé tabuľky
po Live Market Monitore odstránenom v 2.4.2, bez živého kódu. Použiť ich by
znamenalo oživiť odstránenú funkciu.)

Takže badge „Scan failed" by bol vymyslený stav. Nedal som ho tam — rovnaké
odmietnutie ako pri payouts/payments na Calendari. Je na to **test**, ktorý to
stráži.

---

## 5. Last scan info

Per marketplace: najnovší **jeho vlastný** check.
Per event: najnovší **naprieč všetkými** marketplace, aj s listing countom z
toho **istého** checku (nie z iného, novšieho/staršieho).

Porovnanie je textové nad ISO 8601 UTC timestampom — ten sa ako text triedi
identicky ako v čase, takže žiadna konverzia a žiadny timezone posun.

---

## 6. Search / filtre

**Search**: event name, venue, city. Lokálne nad už načítaným zoznamom,
žiadny dotaz.

**Filtre** (segmented control, ten istý ako inde v appke):

| Filter | Znamená |
|---|---|
| **All** | všetko |
| **Needs link** | `linkedCount === 0` |
| **Not scanned** | `checkedCount === 0` |
| **Scanned** | `checkedCount > 0` |

Štyri, nie päť — piaty by musel byť vymyslený (viď vyššie). Žiadny filter
builder.

---

## 7. Check selected — správanie

**Otvorí flow prvého označeného eventu a selekciu nechá.** Zvyšné sú potom
jeden klik každý.

Prečo nie dávka: scanner otvára **reálne viditeľné browser okno**, ktoré si
sám scrolluješ. Neexistuje architektúra na viac eventov naraz a paralelné
sessions si explicitne zakázal. Takže žiadny queue, žiadna automatizácia,
žiadne okno sa neotvorí bez tvojho kliknutia.

Pribudol aj **„← All events"** link — dropdown, ktorý bol dovtedy cestou
späť, už neexistuje.

---

## 8. Zmenené súbory

**Backend (3):**
- `commands/price_checker.rs` — nový `list_price_checker_overview` + 8 testov
- `models.rs` — `PriceCheckerEventOverview`, `PriceCheckerMarketplaceStatus`
- `lib.rs` — registrácia príkazu

**Frontend (3):**
- `pages/PriceChecker.tsx` — `EventOverviewList` + `EventOverviewCard`,
  dropdown odstránený, back link pridaný
- `lib/types.ts` — mirror nových typov
- `lib/api.ts` — `listPriceCheckerOverview`

**DB: 0.** Žiadna migrácia (ďalšia nová je stále **027**), žiadna tabuľka,
žiadny stĺpec, žiadny index, žiadna dependency.

---

## 9. Test results

### 8 nových Rust testov

| Tvoja požiadavka | Test |
|---|---|
| všetky eventy sa zobrazia | `overview_lists_every_upcoming_event_and_no_completed_or_cancelled_one` |
| event bez linkov / nikdy neskenovaný | `an_event_with_no_links_and_no_checks_reports_nothing_rather_than_zeroes` |
| event s linkami / s predošlým scanom | `link_and_check_state_are_reported_per_marketplace` |
| správne last scan info + listing count | `the_newest_check_across_marketplaces_is_the_one_reported_for_the_event` |
| — | `one_events_data_never_leaks_onto_another` |
| — | `a_retired_marketplace_appears_only_on_events_that_really_have_its_data` |
| scan failed state | `no_marketplace_status_can_ever_report_a_failure_because_none_is_stored` |
| no automatic scans / no background | `the_overview_reads_nothing_and_writes_nothing` |

Checkbox selection, select all, clear, multi-select, search a filtre sú
čistý React state v `EventOverviewList` — v tomto projekte nie je frontend
test runner (rovnaká situácia ako pri každom predošlom UI kole), takže sú
overené čítaním, nie testom. Hovorím to rovno.

### Statická kontrola

| | |
|---|---|
| Zátvorky vo všetkých 6 zmenených súboroch | 0 / 0 / 0 |
| Všetkých 42 `#[test]` naviazaných na `fn` | ✓ |
| Importy v `PriceChecker.tsx` | všetky sa rozlišujú |
| Miešané typy v poli (chyba, čo zhodila 2.9.0) | **žiadne pole v novom kóde** |
| `Option::is_none_or` (Rust 1.82, crate má MSRV 1.77) | našiel a nahradil obyčajným `match` |
| Príkaz registrovaný v `lib.rs` + api binding | ✓ |
| `$CommitMsg` bez úvodzoviek | ✓ |

---

## 10. Čo zostalo nezmenené

**Celá scan logika:** parser, readers, DOM scanning, `price_checker_scan.js`,
`price_checker_scanner.rs`, market calculations, tier grouping, section/row
metadata, price history, Your Tickets comparison, Market Analysis.

**Detail eventu je presne ten istý** — klik na event otvorí rovnaký scanner,
rovnaké histórie, rovnaké modaly ako predtým. Tento task menil **iba výber a
prehľad eventov**.

**Pricing pravidlo:** Tier/Level = market grouping, Section/Row/Seat =
metadata. Žiadny pricing podľa section/row, žiadne odporúčania, žiadny
repricing.

**Nepribudlo:** Live Event Intelligence, Live Market Monitor, Auto Monitor,
scheduled scanning, background monitoring, polling, automatic repricing.

Protected areas nedotknuté: refund/resell · `batch_id` · money/integer cents ·
Orders · Tickets · Sales · Listings · Finance · Fulfillment · Attention ·
Calendar · Google Sheets.
