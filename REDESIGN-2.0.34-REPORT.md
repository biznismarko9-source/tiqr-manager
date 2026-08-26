# TIQR Manager 2.0.34 — Date sa už neoreza, Orders aj Events majú Sort

## Čo je nové

1. **Sales - stĺpec Date zobrazuje celý dátum** (napr. "23 Aug 26" namiesto orezaného "23 aug …").
2. **Orders aj Events majú nový "Sort" filter** - presne to, čo Sales už mala, teraz aj na týchto dvoch stránkach, aby sa dalo prehodiť poradie podľa dátumu namiesto pevne daného.

## Date stĺpec (Sales)

Rovnaký typ chyby ako SALE kód v 2.0.33, len na inom stĺpci: `formatDateCompact` vracia napr. "23 Aug 26" (až 9 znakov), na to bolo pri pôvodnej šírke 76px k dispozícii len ~60px textu po odsadení - potrebovalo cca 63-69px, teda tesne pod hranicou. Rozšírené na 92px.

Vedľajší efekt, ktorý treba vedieť: toto je už DRUHÝ krát (po 2.0.33), čo takáto oprava uberá z voľného priestoru pre stĺpec Event - jeho podlaha na najužšom okne appky klesla z 50px na 34px. Stále to nič nerozbíja (Event má vlastný truncate+tooltip aj `overflow-x-auto` poistku), ale je to čoraz tesnejšie a každá ďalšia podobná oprava bude znova uberať odniekiaľ. Práve preto sa aj pýtam v chate, čo chceš s celkovým prístupom k šírke tabuľky - je to presne tá istá téma.

## Sort na Orders a Events

Obe stránky doteraz nemali ŽIADEN spôsob zmeniť poradie - bolo to natvrdo `purchase_date DESC` (Orders) / `event_date DESC` (Events) v Rust backende, bez možnosti to prepnúť. Sales toto UŽ malo (Newest/Oldest/Revenue/Profit/Tickets), tak som len doplnil chýbajúce dva.

**Orders:** nový "Sort" dropdown - "Newest first" (predvolené, presne ako doteraz) / "Oldest first". Triedi sa na frontende nad už načítaným zoznamom, nie novým dopytom na backend - `list_orders_impl` aj tak vždy vráti všetko naraz (do stropu 5000 záznamov), takže klientské triedenie je presne také úplné, ako by bolo serverové, bez toho, aby som musel čokoľvek meniť v `orders.rs`. "Newest first" doslova len vracia zoznam tak, ako prišiel (žiadny prepočet), "Oldest first" ho jednoducho otočí - keďže backend vracia deterministicky zoradený zoznam (`purchase_date DESC, id DESC`), otočenie dá presne `purchase_date ASC, id ASC`, žiadna vlastná logika porovnávania dátumov, teda žiadny priestor na chybu v nej.

**Events:** rovnaký princíp, ale s jedným rozdielom - `event_date` môže byť aj `NULL` (eventy bez dátumu). Nazval som možnosti inak než pri Orders/Sales ("Newest/Oldest") - pri dátume KONANIA eventu (často v budúcnosti) mi "najnovší" neznie jednoznačne, tak je to "Furthest first" (predvolené, presne ako doteraz) / "Soonest first". Eventy bez dátumu ostávajú vždy nakoniec, v oboch smeroch - presne tak, ako to už dnes robí backend, len som tú istú logiku zopakoval aj pre novú možnosť.

Obe zmeny sú čisto frontend, žiadny Rust súbor sa nedotkol - takže žiadne riziko z toho, že v tomto sandboxe neviem `cargo build`/`cargo test` spustiť.

Sales sa v tomto kole nemenila - už mala plnohodnotný sort, nebolo čo dopĺňať.

## FOUND BUT NOT TOUCHED

Popri Date stĺpci som narazil na ešte jeden výskyt úplne rovnakého problému (úzky stĺpec + `truncate` + kód s pevnou dĺžkou), tentoraz na **Order Detail's stĺpci "Ticket"** (kód tiketu, napr. "TIX-000001", 84px - ešte tesnejšie než SALE bolo pred 2.0.33). Spolu so zoznamom z 2.0.33 (Orders.tsx aj Tickets.tsx, stĺpec Order, 92px) mám teraz **tri** miesta čakajúce na tvoje áno:

1. Orders.tsx - stĺpec Order (92px)
2. Tickets.tsx - stĺpec Order (92px)
3. OrderDetail.tsx - stĺpec Ticket (84px)

Je to všade identická oprava (rozšíriť stĺpec) - stačí povedať "áno, oprav všetky tri" a spravím to v ďalšom kole.

## Testy a build

Žiadny Rust súbor sa v tomto kole nezmenil - `cargo test` teda bez zmeny oproti 2.0.32/2.0.33 (494 testov, 491 spustených/passed, 3 ignored). Frontend overený staticky (rovnaké obmedzenie sandboxu ako doteraz - žiadny `npm ci`/`npm run build`/skutočný Playwright): `Sales.tsx`, `Orders.tsx` a `Events.tsx` prešli syntaktickou kontrolou cez `typescript` balík (`ts.transpileModule`) - čisto, žiadne diagnostiky. JSON súbory overené cez `JSON.parse`. Aritmetika stĺpcov v Sales prepočítaná ručne (774px/34px, pozri komentár v kóde). Vizuálne to, prosím, skontroluj sám po `1-CLICK-UPDATE.bat`.

## Zmenené súbory

**Frontend (3 súbory):**
- `src/pages/Sales.tsx` - šírka stĺpca Date (76px → 92px) + aktualizovaný komentár nad `<colgroup>`
- `src/pages/Orders.tsx` - nový Sort dropdown + klientské triedenie
- `src/pages/Events.tsx` - nový Sort dropdown + klientské triedenie (s ošetrením `NULL` dátumov)

**Verzia (8 miest):** ako vždy, všetkých na `2.0.34`.

## Čo sa NEMENILO

Žiadny Rust súbor, žiadna migrácia, `sales.rs`/`orders.rs`/`events.rs` backend logika (sort je čisto frontend), Sales.tsx-in sort (už bol hotový). Zvyšné tri "found but not touched" miesta vyššie.

## Otvorené témy

K téme šírky tabuľky (a jej narastajúcemu napätiu so stĺpcom Event), k notifikáciám pri blížiacom sa evente, a k "Seats" sekcii sa vraciam priamo v chate - všetky tri sú skôr rozhodnutia než opravy, tak sa najprv chcem uistiť, že mierim správnym smerom.

## STOP

2.0.34 hotové. Skontroluj:

1. Sales - dátum by mal byť celý ("23 Aug 26", nie orezaný).
2. Orders aj Events - mal by pribudnúť "Sort" dropdown vedľa "Category", s funkčným prepínaním.
3. Nič iné by sa nemalo líšiť od 2.0.33.
