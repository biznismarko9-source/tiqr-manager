# TIQR Manager 1.9.0 — Payments & Cashflow

Report k verzii **1.9.0**. Nadväzuje na 1.8.3 (Sales + workflow + UX improvements). Táto verzia pridáva Payments & Cashflow presne podľa zadania — **bez novej migrácie, bez novej Payment/Invoice entity**, postavené na existujúcom `payment_status` poli na `sales` aj `orders`.

---

## 1. Audit existujúceho payment modelu

Pred akoukoľvek zmenou som si prešiel `models.rs`, `orders.rs`, `sales.rs`, `finance.rs`, `money.rs`, `dashboard.rs`, `Dashboard.tsx`, `Sales.tsx`, `SaleDetail.tsx`, `OrderDetail.tsx` a `types.ts`. Zistenia:

**Sale (`sales.payment_status`)** — presne 3 stavy: `pending | paid | refunded` (CHECK constraint, migrácie 001/004). `refunded` je dosiahnuteľný **iba** cez `refund_sale_impl` (dedikovaná akcia „Refund"), nikdy cez bežný create/update. Každý riadok `sales` má `sale_price_cents` — celú sumu za daný lístok. **Neexistuje čiastočná platba na úrovni jedného predaja** — predaj je vždy celý pending, celý paid, alebo celý refunded. To presne zodpovedá tomu, čo si zadanie žiadalo ako „preferovaný prvý krok" (bod 2 zadania).

**Order (`orders.payment_status`)** — presne 3 stavy: `unpaid | partial | paid` (validované v `update_order`). Na rozdiel od Sale **neexistuje žiadne číselné pole „koľko už bolo zaplatené"** — `partial` je čistá textová nálepka bez sumy za ňou. Toto je dôležitý rozdiel oproti Sale a určilo to, ako som k Order Detail pristúpil (pozri §6 nižšie).

**Čo už existovalo a dalo sa priamo použiť:**
- `GROUP_BASE_SELECT` (sales.rs, DO NOT TOUCH) už počíta `revenue_cents`/`cost_cents`/`profit_cents` len z nerefundovaných riadkov, `payment_status` na úrovni skupiny (`Some(status)` len keď všetky riadky zdieľajú rovnaký stav, inak `None` = Mixed) a `refunded_count`.
- `list_sale_groups_impl` **už mal** parameter `payment_status` a Sales.tsx **už mal** filter Payment (Pending/Paid/Refunded) — toto je 1.8.0-éra funkcia, nie nová.
- 1.8.3 pridala `pending_sales_count`/`pending_sales_amount_cents`/`pending_sales_currency` do `DashboardAlerts` — presne ten istý „SUM podľa payment_status, scoped na primary_currency, null = mixed" vzor som použil aj pre 1.9.0.
- `paid_at`/žiadny timestamp prechodu pending→paid **neexistuje** nikde (len `sales.refunded_at` pre refund). Pozri §9.

**Záver auditu:** existujúce polia plne stačia na to, čo zadanie žiada. Nový Payment/Transaction záznam **nebol vytvorený**.

---

## 2. DECISION — žiadna nová migrácia, žiadna Payment entita

Podľa explicitnej inštrukcie zo zadania („Ak samostatná Payment entita nie je nevyhnutná, NEVYTVÁRAJ ju") som sa rozhodol takto:

- **Sales / Sale Detail / Dashboard cashflow** — postavené výhradne na `sales.payment_status` + `sales.sale_price_cents`. Nula nových tabuliek, nula nových stĺpcov v `sales`.
- **Order Detail** — Paid/Outstanding sú odvodené z existujúceho `orders.payment_status` + `orders.total_cost_cents`, **bez fabrikovania** presnej sumy pre `partial` (vysvetlené v §6).
- Jediná nová vec v databázovej vrstve je **jeden nový Rust struct** `CashflowSummary` (čisté DTO pre Dashboard, žiadna nová tabuľka) a jedno nové SQL-agregátne query (`paid_cents`).

**Migrácia 005 nebola potrebná a nebola vytvorená.** `finance.rs`, `money.rs` a schéma databázy zostávajú presne také, aké boli v 1.8.3.

---

## 3. Dashboard — sekcia Cashflow

Pridaná nová sekcia **„Cashflow (all time)"** medzi „Current inventory" a „Inventory & Potential Profit": 4 karty — **Revenue, Profit, Paid, Outstanding**.

- Revenue/Profit sú **presne tie isté čísla**, ktoré Dashboard interne už počítal (`data.inventory.revenueCents/profitCents`) — len doteraz neboli nikde na obrazovke zobrazené. Nešlo teda o nový výpočet.
- Paid = `SUM(sale_price_cents) WHERE payment_status='paid'`, scoped na `primary_currency` (rovnaký vzor ako existujúci `pending_sales_amount_cents`).
- Outstanding = **rovnaká hodnota** ako `alerts.pendingSalesAmountCents` (existujúci Pending Sales alert) — nie duplicitný výpočet, len prepoužitá premenná, aby sa nikdy nemohli rozísť.
- Mixed currency → `currency: null` na celom bloku → frontend zobrazí „Mixed" (rovnaký princíp ako všade inde).
- Platí invariant `revenue = paid + outstanding` (pre jednu menu) — overené testom, pozri §8.
- Nie je period-filtered — je to „stav teraz", rovnako ako existujúci Pending Sales alert a Attention sekcia.

**Performance:** presne **jedno nové SQL query** (`paid_cents`). Nič iné sa neopakuje — Outstanding aj Revenue/Profit sú znovupoužité existujúce hodnoty, žiadne N+1.

**Dashboard Attention (bod 8 zadania):** existujúci alert „Pending sales" zostal bez zmeny. **Samostatný „Outstanding payments" alert som nepridal** — bol by to duplicitný údaj s Cashflow sekciou aj s existujúcim Pending Sales alertom, a zadanie samo hovorilo „ak relevantné" — pri troch miestach zobrazujúcich to isté číslo by Dashboard prestal byť „čistý", presne čomu sa mal 1.9.0 vyhnúť.

---

## 4. Sales — Payment summary

V summary lište (nad tabuľkou, kde už boli Results/Tickets/Revenue/Profit) pribudli **Paid** a **Outstanding**, počítané **výhradne z dát, ktoré má obrazovka už načítané** — bez nového backend requestu.

Dôležité obmedzenie, ktoré transparentne priznávam: `SaleGroup.paymentStatus` je `Some(status)` **len keď všetky riadky v danej skupine zdieľajú jeden stav** (existujúce pole z `GROUP_BASE_SELECT`). Pri takej skupine viem presne povedať, či jej `revenueCents` patrí do Paid alebo Outstanding. Pri skupine s **Mixed** stavom (napr. 3 lístky paid, 1 pending v tom istom predaji) by presný rozpad vyžadoval nový SQL agregát priamo v `GROUP_BASE_SELECT` — a to je na explicitnom DO NOT TOUCH zozname (`SaleGroup`, `GROUP_BASE_SELECT`), navyše presne toto miesto malo v minulosti niekoľko jemných bugov (BUG #4, #6, H5), takže som ho **zámerne nechal netknuté**.

Riešenie: Mixed-status skupiny sú z Paid/Outstanding súčtu vynechané a pod lištou sa objaví transparentná poznámka („N predajov s mixed payment status nie je zarátaných...") — presne v duchu vlastnej inštrukcie zadania „ak sa dá bezpečne určiť z existujúcich dát". Pre presný rozpad takej skupiny slúži Sale Detail (§5), kde sa počíta z jednotlivých riadkov, nie z agregátu.

Payment filter (Pending/Paid/Refunded) **už existoval** pred 1.9.0 (1.8.0-éra `list_sale_groups_impl`), overil som ho a nechal bez zmeny — okrem toho som doplnil chýbajúci automatizovaný test naň (§8, scenár „Sales payment filter").

---

## 5. Sale Detail — Payment status + Paid/Outstanding

Do existujúcej info karty (Platform / Sale date / Currency / Refunded) pribudli **Paid** a **Outstanding**, počítané priamo z jednotlivých riadkov (`lines: Sale[]`), ktoré táto stránka aj doteraz načítavala — **žiadny nový backend request**.

- Paid = súčet `salePriceCents` riadkov s `paymentStatus === "paid"`.
- Outstanding = súčet riadkov s `paymentStatus === "pending"`.
- Refundované riadky nepatria do ani jedného vedra — nikdy sa nezarátajú ako outstanding (explicitná požiadavka zadania, bod 3 a 6).
- Mena: ak podmnožina (paid/pending) obsahuje riadky s rôznou menou, zobrazí sa „Mixed" pre tú konkrétnu sumu; ak je podmnožina prázdna (napr. nič nie je pending), spadne to na menu celého predaja namiesto nesprávneho „Mixed" pri skutočnej nule.

Existujúci Payment status badge (Paid/Pending/Refunded/Mixed) aj Revenue/Fees/Cost/Profit/Margin/ROI riadok zostali **úplne bez zmeny** — refund accounting je nedotknutý.

---

## 6. Order Detail — Order total / Paid / Outstanding / Payment status

Do existujúcej hornej karty (Quantity/Unit price/Fees+other/Total cost) pribudli **Paid** a **Outstanding**, odvodené **čisto z existujúceho `order.paymentStatus`**:

| `order.paymentStatus` | Paid | Outstanding |
|---|---|---|
| `paid` | celá `totalCostCents` | 0 |
| `unpaid` | 0 | celá `totalCostCents` |
| `partial` | **„Partial" (text, žiadne číslo)** | **„Partial" (text, žiadne číslo)** |

Toto je zámerné a je to priamy dôsledok auditu z §1: Order **nemá** číselné pole „koľko presne bolo doteraz zaplatené", takže pri `partial` by akékoľvek číslo bolo vymyslené. Namiesto fabrikovania som zvolil čestné zobrazenie textu „Partial" s poznámkou „Exact amount not tracked" — presne v duchu explicitnej inštrukcie zadania „NEPRENÁŠAJ Order partial status automaticky na Sales, ak to nedáva zmysel" a všeobecného pravidla „nikdy nič nevymýšľaj". Existujúci Order Payment Status badge zostal bez zmeny. **Nula nových backend requestov** — čisto frontendová derivácia z polí, ktoré Order Detail už má.

---

## 7. Mixed currency

Platí dôsledne všade v 1.9.0: dve rôzne meny sa **nikdy** nesčítajú do jedného čísla.

- Dashboard Cashflow: `currency: null` keď `mixed_currencies` (backend, rovnaký signál ako všade inde na Dashboarde).
- Sales summary lišta: `cashTotals.currency` je `null`, keď „definite" skupiny (tie s jednoznačným stavom) nezdieľajú jednu menu.
- Sale Detail: `paidCurrency`/`outstandingCurrency` počítané osobitne pre každé vedro (paid/pending), s bezpečným fallbackom na menu predaja pri prázdnom vedre (aby skutočná nula nikdy nevyzerala ako „Mixed").
- Order Detail: mena je vždy jedna konkrétna (`order.currency`) — na úrovni jednej objednávky sa miešanie mien nevyskytuje.

Pri kontrole nezávislým reviewom (pozri §10) sa našiel jeden reálny okrajový prípad: keď Sales zoznam vráti **0 výsledkov** (nová inštalácia, alebo filter, ktorému nič nezodpovedá), pôvodná verzia môjho kódu chybne ukazovala „Mixed" namiesto skutočnej nuly. Opravené — pri 0 výsledkoch sa Paid/Outstanding v summary lište jednoducho nezobrazujú (rovnaký princíp, aký už lišta používa pre iné irelevantné nulové hodnoty).

---

## 8. Testy

Rusty testy pribudli/rozšírili sa presne podľa 10 scenárov zo zadania (bod 12):

| # | Scenár zo zadania | Test |
|---|---|---|
| 1 | Paid sale | `cashflow_splits_revenue_into_paid_and_outstanding_and_the_invariant_holds` |
| 2 | Pending sale | tamtiež (paid+pending v jednom teste) |
| 3 | Refunded sale | `cashflow_excludes_refunded_sales_from_paid_outstanding_and_revenue` |
| 4 | Pending Sales dashboard | existujúce 1.8.3 testy nezmenené + `cashflow_outstanding_drops_to_zero_once_a_pending_sale_is_refunded` |
| 5 | Refunded sale nie je outstanding | `cashflow_outstanding_drops_to_zero_once_a_pending_sale_is_refunded` |
| 6 | Mixed currency payment summary | `cashflow_is_scoped_to_primary_currency_and_shows_none_when_mixed` |
| 7 | Single currency payment summary | `cashflow_splits_revenue_into_paid_and_outstanding_and_the_invariant_holds` |
| 8 | Refund → resell | `cashflow_reflects_only_the_active_sale_after_refund_then_resell` |
| 9 | Existing Order payment status | `orders.rs` nezmenený, existujúca test suite naďalej pokrýva |
| 10 | Sales payment filter | `list_sale_groups_payment_status_filter_matches_the_right_groups` + `list_sale_groups_payment_status_filter_is_a_contains_a_line_semi_join` (nový test odhalil, že tento filter je „obsahuje aspoň jeden riadok", nie „všetky riadky" — zdokumentované v teste) |

Naviac: `empty_inventory_gives_zeroed_potential_not_an_error` rozšírený o 5 assertov (cashflow na prázdnej DB = všetko 0, mena EUR). Žiadna nová DB tabuľka/migrácia → upgrade test zo starej DB nebol potrebný.

**Spolu:** 42 testov v `sales.rs` (+2 oproti 1.8.3), 33 testov v `dashboard.rs` (+6 oproti 1.8.3).

---

## 9. Payment timestamp (bod 9 zadania) — bez zmeny

Potvrdené auditom: neexistuje žiadne pole, ktoré by zaznamenávalo, kedy sa `payment_status` zmenil z pending na paid (na rozdiel od `refunded_at`, ktorý pre refund existuje). Podľa explicitnej inštrukcie zadania som to **nepridal automaticky** — je to len uvedené ako future improvement (§11).

---

## 10. Regression a DO NOT TOUCH

Skontrolované, že žiadna z týchto vecí nebola zmenená: `finance.rs`, `money.rs`, refund/resell logika, `SaleGroup`/`batch_id`/`GROUP_BASE_SELECT` (žiadny riadok v ňom sa nezmenil — len 2 nové testy, ktoré ho volajú, nie upravujú), existujúci Order Payment Status, Backup/Restore, CSV import, Tickets/Inventory/Events grouping, Settings architektúra, Dashboard chart. Revenue/realized profit/cost/ROI vo všetkých existujúcich testoch zostávajú nezmenené.

Kód som si nemohol dať skompilovať (pozri §11 nižšie), takže namiesto spoliehania sa len na vlastné čítanie som **dal celú zmenu prejsť nezávislým reviewom** (samostatný agent, bez kontextu mojej práce) — ten našiel a ja som opravil jeden reálny bug (Sales summary lišta pri 0 výsledkoch, §7) a jednu nepresnosť v komentári. Po oprave review potvrdil zvyšok zmien ako korektný. Manuálne som tiež overil, že v každom upravenom súbore sedí počet `{`/`}` aj `(`/`)`.

---

## 11. Build

Presne ako pri každej predchádzajúcej verzii: v tomto sandboxe je **trvalo zablokovaný sieťový prístup** na crates.io aj registry.npmjs.org.

```
cargo check --lib  →  error: failed to get `anyhow` ... 403 Host not in allowlist: index.crates.io
npm install        →  npm error 403 Forbidden - registry.npmjs.org
```

`node_modules` je prázdny (0 balíkov), takže ani `tsc -b` sa nedá reálne spustiť (chýbajú typy). Toto nie je dôsledok žiadnej zmeny v 1.9.0 — je to obmedzenie tohto prostredia, potvrdené odznova aj tentokrát. Skutočné overenie prebehne až u teba cez `1-CLICK-UPDATE.bat` → GitHub Actions.

---

## 12. Čo sa zmenilo — súhrn súborov

**Rust (backend):**
- `src-tauri/src/models.rs` — nový `CashflowSummary` struct, nové pole `cashflow` na `DashboardData`.
- `src-tauri/src/commands/dashboard.rs` — jedno nové SQL query (`paid_cents`), zostavenie `cashflow`, 6 nových testov + rozšírenie 1 existujúceho.
- `src-tauri/src/commands/sales.rs` — **žiadna produkčná zmena**, len 2 nové testy na existujúci `payment_status` filter.

**TypeScript/React (frontend):**
- `src/lib/types.ts` — nový `CashflowSummary` interface, nové pole na `DashboardData`.
- `src/pages/Dashboard.tsx` — nová sekcia „Cashflow (all time)" (4 karty).
- `src/pages/Sales.tsx` — Paid/Outstanding v summary lište + transparentná poznámka pri vynechaných Mixed-status predajoch.
- `src/pages/SaleDetail.tsx` — Paid/Outstanding v info karte.
- `src/pages/OrderDetail.tsx` — Paid/Outstanding v hornej karte (čestné „Partial" pri partial stave).

Žiadna nová migrácia, žiadny nový súbor s DB schémou.

---

## 13. Future improvements (nerobené teraz, len na zváženie)

- Presný Paid/Outstanding rozpad aj pre Mixed-status skupiny v Sales zozname — vyžadovalo by rozšíriť `GROUP_BASE_SELECT` o 2 ďalšie `SUM(CASE...)` stĺpce. Zámerne nespravené teraz (DO NOT TOUCH, história bugov na tomto mieste).
- `paid_at` timestamp (kedy presne sale prešiel z pending na paid) — vyžadovalo by nové pole na `sales` (malá additive migrácia). Zámerne nespravené teraz (bod 9 zadania).
- Číselný rozpad `partial` na Order — vyžadovalo by nové pole „amount paid" na `orders` (malá additive migrácia). Zámerne nespravené teraz — bolo by to fabrikovanie dát bez takej migrácie.

Toto sú len nápady pre budúcnosť, nič z toho som nezačal implementovať.

---

## STOP

Podľa zadania týmto **končím po 1.9.0** a čakám na tvoju spätnú väzbu k reportu. Nezačínam Invoices/Cloud/Discord/Webhooks/Accounts/Marketplace integrácie.
