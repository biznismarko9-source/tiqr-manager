# TIQR 2.26.0 — Price Checker Visual Market Map

---

## 1. Čo marketplace REÁLNE poskytujú

Toto som zistil **čítaním readera**, nie odhadom — a zmenilo to návrh.

**Najdôležitejší nález: nie sú tri readery.** `price_checker_scan.js` je **jeden
generický DOM reader**. Funkcie `readStubHub` / `readVividSeats` / `readTicombo`
volajú **presne ten istý** `scanWithSelectors(LISTING_PRICE_SELECTORS, ...)` —
líšia sa **iba menovkou**, ktorú nalepia na listing. Rozdiel medzi Viagogom,
Vivid Seats a Ticombom v extrakcii **neexistuje**.

Čo reader vie z jedného listingu dostať:

| údaj | dostupné? | ako |
|---|---|---|
| cena | **áno** | parser peňazí + pravidlá odmietania (preškrtnuté, fees, page chrome) |
| mena | best-effort | zo symbolu/kódu; `null` keď je nejednoznačná (napr. holé „kr") |
| **section** | best-effort | `/\bsec(?:tion)?\.?\s*[:#]?\s*(...)/i` |
| **row** | best-effort | `/\brow\.?\s*[:#]?\s*(...)/i` |
| **tier / level** | best-effort | inline regex ALEBO najbližší nadpis nad blokom |
| quantity | best-effort | `qty`, `x 2`, `2 tickets` |
| listing ID | zriedka | `id` / `data-*` atribút, ak ho markup má |
| marketplace | **áno** | z hostname |

Čo reader **neposkytuje vôbec, v žiadnom marketplace**:

- **seat number** — v celom scriptte **nie je žiadny seat pattern**. Neexistuje.
- **venue** — nečíta sa.
- **mapové dáta** — nič nečíta seat-map widget stránky. **Žiadne súradnice,
  žiadne polygóny, žiadna skutočná susednosť section.**
- **URL konkrétneho listingu** — nezachytáva sa žiadny `href`.
- **availability** — iba `quantity`.

**Preto mapa nie je geometrická replika a ani sa tak netvári.** Presne ako si
napísal: nepredstieram presnú mapu, použil som dostupné section/level dáta a
tu je to napísané.

---

## 2. Bug, ktorý audit našiel — a opravil

**`hostFamily` nemala vetvu pre viagogo.**

Viagogo nahradilo StubHub ako seedovaný marketplace v migrácii **017**, StubHub
bol úplne odstránený v **020** — ale tento súbor sa nikdy neaktualizoval. Takže
**každý scan viagogo stránky prepadol na `"generic"`** a všetky jeho listingy
dostali menovku `"generic"`.

Keďže sa tri readery líšia iba menovkou, oprava je menovka — tri riadky:
`readViagogo`, vetva v `hostFamily`, vetva v dispatchi. **Nič iné v scanneri som
nechytal.** Bez toho by bol marketplace filter na mape nepoužiteľný.

---

## 3. Čo sa dostalo do mapy

Nový príkaz **`compute_market_map`** — zámerne **presne taký tvar ako
`compute_market_analysis` vedľa**: prečítaj listingy, ktoré session už má,
prečítaj tvoje nepredané lístky na ten event, spoj, vráť.

**Žiadny nový scanner. Žiadna nová tabuľka. Žiadna migrácia. Žiadne pozadie,
žiadny polling, žiadny auto-scan.** Scanner lifecycle (open / scan / cancel /
close) je nedotknutý.

Mapa: **pásy tierov → bloky sections**. Blok je tónovaný v **štyroch krokoch**
podľa počtu listingov — nie plynulý gradient, lebo to je „tu je ich viac než
tam", nie meranie.

---

## 4. Normalizačné pravidlá

| vstup | výsledok |
|---|---|
| `Sec 102`, `Section 102`, `102` | jeden blok, key `102` |
| `0102` | `102` (nuly dolu **iba** keď je zvyšok celý číslo) |
| `0A` | `0A` — nedotknuté |
| `FLOOR A` | `FLOOR A` — nedotknuté |
| `Row 7`, `07` | row `7` |
| `AA` | `AA` — nedotknuté |
| `Level 100` vs `Tier 1` | **DVE samostatné skupiny** |

**Každá skupina si drží oboje:** `key` (podľa čoho sa zoskupuje) a `label`
(**prvá zdrojová hodnota**, ktorá tam padla — to sa kreslí). Hodnota, ktorú sa
nedá bezpečne zredukovať, **sa neprepisuje** — stane sa vlastnou skupinou pod
vlastným textom.

**Tier sa nikdy nemapuje.** „Level 100" a „Tier 1" ostávajú oddelené, lebo táto
appka nemá ako vedieť, či znamenajú to isté.

---

## 5. My Tickets overlay

Tvoje **available/listed** lístky na ten event (rovnaký scope, aký už používa
`your_tickets` v Market Analysis) idú na **tie isté bloky**. Sú:

- **počítané oddelene** — nikdy sa nezlejú do market countu,
- označené smaragdovou bodkou na bloku a smaragdovým riadkom v detaile,
- **nesú `ticket_id` aj `order_id`**, takže klik otvorí **existujúci order
  detail**. Žiadny nový ticket-detail systém nevznikol.

Section, kde máš lístky ale trh nemá nič, **sa aj tak zobrazí** — prázdna,
čiarkovaná, s „no listings".

---

## 6. Section detail

Klik na blok:

```
SECTION Sec 102
Market listings: 23 · Your tickets: 2
Lowest €175.00   Median €188.50   Highest €240.00

Row | Seats | Marketplace | Price   | Mine
18  | 5     | TKT-000014  | €190.00 | Yes
18  | 6     | TKT-000015  | not listed | Yes
12  | 2     | viagogo     | €180.00 | No
14  | 4     | vividseats  | €195.00 | No
```

**„Seats" pri market listingu je POČET sedadiel, nie číslo sedadla** — a je to
tam napísané. Tvoje vlastné lístky ukazujú **skutočné** sedadlo, lebo appka ho
ukladá. Tá asymetria je reálna a je pomenovaná.

Filtre: každý marketplace, My tickets, tier. Zoom 70–160 %, scroll je uzavretý
v mape (nescrolluje sa celá stránka).

---

## 7. Čo sa NEDÁ získať — a čo som preto neurobil

- **Seat-level mapa neexistuje.** Reader nemá seat pattern. Sedadlá by som
  musel vymyslieť.
- **Geometria venue neexistuje.** Bloky sú zoradené numericky-potom-abecedne.
  Je to stabilný spôsob, ako section nájsť — **nie tvrdenie, kde v budove je.**
- **Listing nemá vlastnú URL.** Ponúkam jedinú, ktorá existuje: link eventu na
  marketplace, z ktorého scan pochádzal — a je označený ako taký.
- **CURRENT vs PREVIOUS na úrovni section NEEXISTUJE.** Toto je podstatné:
  existujúca Price Checker history (`price_checks` + `price_check_tiers`)
  ukladá **iba agregáty** — lowest/median/highest/count celkovo a per tier.
  **Žiadna section sa do histórie nikdy neukladala.** Porovnanie dvoch scanov
  po sectionoch by si vyžiadalo novú tabuľku — a to si zakázal. Takže mapa
  ukazuje **aktuálny stav session**, ktorá sa po každom ďalšom manuálnom scane
  aktualizuje (session listingy sa kumulujú, presne ako doteraz).
- **Scan bez sections** → mapa to povie a nič nenakreslí. Listingy bez section
  idú do **jedného označeného koša mimo mriežky**, nikdy rozhádzané do blokov.
- **Zmiešané meny v jednej section** → **žiadne** lowest/median/highest. Nie
  blend, nie prepočet.

---

## 8. Žiadny pricing

Lowest / median / highest **opisujú, čo je v tom bloku vypísané**. Nič
neporovnáva jednu section s druhou, nič neinterpoluje medzi susedmi, nič
neodporúča cenu. Section/row/seat sú metadata, tier je market grouping.

---

## 9. Testy

**13 nových testov** v `price_checker_map.rs`: normalizácia section (13
prípadov vrátane `0102`/`0A`/`000`/prázdne/`Section` samotné), normalizácia
row, tri zápisy jednej section → jeden blok, tier sa nemapuje, kôš bez section,
scan úplne bez sections, My Tickets overlay + ID, section iba s mojimi
lístkami, zmiešané meny, poradie blokov, viac marketplace v jednej section,
prázdna session, median párneho počtu.

**Skutočne spustené tu:**
- **Celá čistá logika modulu prenesená a vykonaná** proti všetkým tvrdeniam
  tých 13 testov — **38 assertions, 38 prešlo**. Toto nie je čítanie kódu,
  toto je jeho beh.
- **SQL pre My Tickets spustené proti skutočnej SQLite** so zaseknutým lístkom
  — vracia presne tých 10 stĺpcov v poradí, v akom ich `read_my_tickets` číta.
- **Mapa vykreslená v prehliadači** s presnými triedami: tier pásy, tónovanie,
  vybraný blok, blok kde mám lístky ale trh nič, detail panel.
- **181 príkazov** sedí medzi `api.ts` a `lib.rs` v oboch smeroch.
- Rust: 13× `#[test]` správne na `fn`, zátvorky vyvážené, žiadne zatúlané
  úvodzovky v stringoch. TS/JSX: oba nové aj dotknuté súbory vyvážené.

**Nespustené tu:** `cargo test --lib`, `cargo check --lib`, `npx tsc -b`,
`npm run build` — na tomto Macu nie je Rust ani Node. Prejdú až v CI.

**Regresia:** nedotknuté ostali `price_checker_scanner.rs` (okrem ničoho),
`price_checker_analysis.rs`, `price_checker.rs`, tier grouping, Your Tickets
comparison, manual scanner, Market Analysis. Jediná zmena v scanneri je tá
trojriadková viagogo menovka.

---

## 10. Výkon a limity

- **Jeden DOM blok na SECTION** (desiatky), nikdy jeden na listing (stovky).
- **Riadky listingov sa vykresľujú iba pre otvorenú section.**
- Filtre sú **odvodený stav** nad tými istými poľami — listingy sa nekopírujú
  do druhého úložiska.
- Žiadna nová grafická závislosť. Žiadne canvas/SVG — obyčajné bloky stačia.
- Mapa má vlastný scroll s `overscroll-behavior: contain`, takže sa nescrolluje
  celá stránka.

---

## 11. Zmenené súbory

| súbor | čo |
|---|---|
| `src-tauri/src/commands/price_checker_map.rs` | **nový** — normalizér, fold, 1 príkaz, 13 testov |
| `src-tauri/src/models.rs` | `MarketMap` + 4 štruktúry |
| `src-tauri/src/commands/price_checker_scan.js` | **viagogo menovka** (3 riadky) |
| `src-tauri/src/commands/mod.rs`, `lib.rs` | registrácia |
| `src/components/MarketMapView.tsx` | **nový** — mapa, filtre, detail panel |
| `src/pages/PriceChecker.tsx` | stav + effect + render (rovnaký vzor ako analysis) |
| `src/lib/types.ts`, `src/lib/api.ts` | typy + `computeMarketMap` |
| `CHANGELOG.md`, `PROJECT_STATE/*` | dokumentácia |

**Nedotknuté:** refund/resell, `batch_id`, peniaze, Orders, Tickets, Sales,
Listings, Finance, Fulfillment, Attention, Calendar, Google Sheets, Sync, AI
Import. **Žiadna zmena schémy, žiadna migrácia (ďalšia je stále 029), žiadna
nová závislosť.**
