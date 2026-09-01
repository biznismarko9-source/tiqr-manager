# TIQR Manager 2.2.6 — Inventory Intelligence pre Event Workspace

Reagujem na tvoju správu:

> *"Pome spraviť ďalší focused task: Inventory Intelligence pre Event Workspace. [...] Do Event Workspace →
> Overview pridaj kompaktný "Inventory Intelligence" blok nad existujúci inventory obsah. Zobraz: Total
> tickets, Total invested, Current listed value, Potential profit, Sell-through %, Average ticket cost.
> Aging: 0–7 dní, 8–30 dní, 30–60 dní, 60+ dní. Attention: event <48h + unsold tickets, ticket bez listing
> price, ticket bez active listing, výrazne mimo market ceny (iba ak už existujú dáta z Price Checker/Market
> Analysis). Breakdown: podľa tier, podľa section, podľa marketplace. Dôležité: používaj existujúcu databázovú
> logiku a integer cents, žiadne nové duplicity výpočtov money logiky, používaj existujúce listing/sales/
> market dáta, nič nemen na refund/resell, batch_id, Orders/Tickets/Sales/Inventory core logike, Finance page
> nechaj nedotknutú, žiadny nový veľký redesign Event Workspace. Klikateľnosť: klik na KPI/attention položku
> má otvoriť existujúci relevantný zoznam/filtrovaný view. Ak niektorý údaj už databáza spoľahlivo nevie
> vyrátať, nevymýšľaj fallback dáta — reportuj čo presne chýba."*

Tento raz si v závere nespomenul verziu/report/balík, tak som prvé kolo poslal bez nich (len kód + testy +
dva stavové dokumenty) a v chate sa spýtal, či si to tak naozaj chcel — potvrdil si, že file chceš tiež, takže
toto je ten normálny release: verzia zvýšená, zabalené, s týmto reportom.

Postupne, po poriadku.

## 1. Nový blok na Overview — "Inventory Intelligence"

Zobrazuje sa hneď nad tabuľkami Orders/Tickets (ktoré tam boli od 2.2.4), presne tam, kam si chcel. Žiadny
nový tab, žiadny redesign zvyšku Event Workspace — Listings a Sales sú úplne nezmenené.

## 2. KPIs

- **Total tickets, Total invested** — rovnaký rozsah ako doterajšie čísla na Overview (`finance::compute_summary`
  — všetky tikety vrátane zrušených).
- **Current listed value** — súčet CEN aktívnych záznamov v `ticket_listings` (rovnaká definícia, akú už
  používa karta "Listed value" v Listings).
- **Potential profit** — rovnaký vzorec, aký už roky používa karta "Potential Profit" v Sales (legacy pole
  `tickets.listing_price_cents`).
- **Sell-through %** — predané / všetky tikety (vrátane zrušených — rovnaký menovateľ ako "Total tickets"
  hneď vedľa).
- **Average ticket cost** — priemer z rovnakých nákladov, čo aj "Total invested".

Žiadne z týchto čísel nie je nová výpočtová logika — všetko je buď priama SQL agregácia nad rovnakými
tabuľkami/poľami, čo už používajú Listings/Sales karty, alebo volanie tej istej `finance.rs` funkcie, čo
používa Dashboard/Events/Orders/Sales.

**Jedna vec, čo si treba uvedomiť:** "Current listed value" a "Potential profit" úmyselne NIE sú tá istá
databázová vec. V appke bežia paralelne dva systémy cien — starší jediný stĺpec `tickets.listing_price_cents`
(z neho žije "Potential profit" a aj Price Checkerova "Market vs. mine" karta) a novší, reálny systém
`ticket_listings` z 2.2.4 (z neho žije "Current listed value" aj celý Listings tab). Obe čísla sú reálne a
niekde v appke sa už zobrazujú presne takto — nezjednotil som ich, lebo to nebolo súčasťou zadania a mohlo by
to spôsobiť, že jedno z čísel prestane sedieť s tým, čo vidíš inde. Ak by si chcel tieto dva systémy
zjednotiť naozaj, je to samostatná, väčšia úloha.

## 3. Aging (nepredané tikety podľa veku)

0–7 / 8–30 / 31–60 / 61+ dní od dátumu nákupu objednávky. **Malá oprava oproti tvojmu zadaniu:** napísal si
"8–30" a "30–60", čo sa prekrýva na dni 30 — urobil som z toho 31–60, aby každý nepredaný tiket spadol presne
do jedného koša, nikdy do dvoch naraz.

## 4. Attention

- **Event do 48 hodín + nepredané tikety** — keďže `event_date` v appke nemá nikde uloženú hodinu (len
  dátum), preložil som "48h" na "dnes, zajtra alebo pozajtra" (2 kalendárne dni). Ak si chcel presnejšie
  hodinové okno, appka by najprv potrebovala aj čas eventu, nielen dátum.
- **Ticket bez listing price** — nepredaný tiket bez vyplnenej ceny (`listing_price_cents`).
- **Ticket bez active listing** — nepredaný tiket, ktorý nemá žiadny aktívny záznam v `ticket_listings`.
- **Výrazne mimo market ceny** — použil som presne tú istú funkciu, čo už počíta Sales kartu "Market vs.
  mine" (`get_price_checker_summary`). Keď pre event ešte nie sú žiadne uložené Price Checker dáta, appka to
  ukáže ako "nie sú dostupné dáta", nie fingovanú nulu — presne ako si žiadal ("nevymýšľaj fallback dáta").
  Hranicu "výrazne" som nastavil na 20 % odchýlku od priemernej trhovej ceny — konkrétne číslo si nezadal, je
  to moje rozhodnutie a dá sa jednoducho zmeniť (jedna konštanta v kóde).

## 5. Breakdown

Podľa section a podľa marketplace — obe fungujú a sú klikateľné.

**Podľa tier — toto sa NEDÁ vyrátať, tak som to nevymyslel.** V appke dnes nikde neexistuje stĺpec
tier/level pre tikety. `ticket_type` vyzerá podobne, ale je to spôsob DORUČENIA (E-ticket/PDF/Mobile
transfer/Physical/Will call), nie cenová kategória — použiť ho ako tier by dávalo nezmyselné skupiny. Blok
preto priamo v UI píše, že tier sa zatiaľ nesleduje, namiesto toho, aby som predstieral dáta, ktoré appka
nemá. Najmenšia reálna oprava: nový nepovinný stĺpec `tickets.tier` + miesto v Add/Edit ticket formulári (a v
CSV importe) na jeho zadanie — je to skutočná migrácia, nie rýchla úprava, tak som to nerobil bez tvojho
potvrdenia.

## 6. Klikateľnosť

Každé KPI, každý aging košík aj každá attention položka sa dá kliknúť — prefiltruje to Tickets tabuľku
priamo na Overview presne na tie tikety, o ktoré ide (s malým pásikom hore "Showing: ..." a odkazom na
zrušenie filtra). Výnimka je "Current listed value" — to prepne na existujúci Listings tab, lebo ide o
`ticket_listings` záznamy, nie o samotné tikety.

Nešiel som cez Tickets.tsx ani Orders.tsx stránky, pretože ani jedna dnes nemá spôsob, ako ich otvoriť už
prefiltrované na konkrétne ID (Tickets.tsx vie len `?code=` pre jeden tiket, Orders.tsx len predvyplní formulár
na nový order). Filtrovanie preto zostáva celé v rámci tejto jednej stránky (Event Workspace), presne v
duchu "žiadny nový veľký redesign".

## Čo som overil

```
cargo test --lib   -> 972 passed, 0 failed, 3 ignored (+13 nových testov pre inventory_intelligence:
                       zhoda KPI s existujúcim finance/Listings/Sales rozsahom, hranice aging košíkov,
                       nezávislosť attention položiek, currency-mixed handling, section/marketplace grouping)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.6 build" v hlavičke)
```

Skontroloval som aj rozsah zmien priamo súbor po súbore — Listings, Sales, Finance stránka ani zvyšok Event
Workspace neboli zasiahnuté (žiadna migrácia, žiadna nová závislosť, presne 7 upravených/nových súborov).

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/inventory_intelligence.rs` — nový súbor, 1 nový príkaz (`get_inventory_intelligence`)
  + 13 nových testov
- `src-tauri/src/models.rs` — nové DTO (`InventoryIntelligence`, `InventoryIntelligenceKpis`, `AgingBucket`,
  `AttentionItem`, `InventoryBreakdownGroup`)
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia nového príkazu

**Frontend:**
- `src/pages/EventDetail.tsx` — nový blok na Overview + lokálne filtrovanie Tickets tabuľky podľa kliku
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a API funkcia

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

**Nezmenené (podľa tvojho "nič nemen"):** refund/resell, `batch_id`, Orders/Tickets/Sales/Inventory core
logika, Finance stránka, žiadna nová migrácia, žiadna nová závislosť.

**Verzia (9 miest v 7 súboroch):** `2.2.6`.

## Limity, ktoré treba vedieť

- **Breakdown podľa tier chýba úplne** — appka nemá tier/level stĺpec, pozri bod 5 vyššie. Ak ho chceš
  reálne, treba novú migráciu + úpravu formulárov.
- **"48h" = 2 kalendárne dni**, nie presné hodinové okno (appka nemá čas eventu, len dátum).
- **20 % odchýlka od trhovej ceny** je moje rozhodnutie, nie tvoje zadané číslo — ľahko zmeniteľné.
- **Sell-through % počíta zo všetkých tiketov vrátane zrušených** — ak by si chcel bez zrušených, daj vedieť.

## STOP

2.2.6 hotové, otestované a zabalené. Skontroluj:

1. **Event Workspace → Overview** — nad tabuľkami Orders/Tickets by si mal vidieť nový blok "Inventory
   Intelligence" so 6 KPI, aging riadkom, attention zoznamom a breakdown podľa section/marketplace.
2. **Klikni na pár KPI/aging/attention položiek** — over, že sa dole prefiltruje Tickets tabuľka na správne
   tikety a že sa dá filter zrušiť. Klikni aj na "Current listed value" — mal by ťa prepnúť na Listings tab.
3. **Event bez Price Checker dát** — over, že "výrazne mimo market ceny" ukazuje "nie sú dostupné dáta", nie
   nulu.
4. **Event s eventom o pár dní** — over, či ti sedí preklad "48h" na "dnes/zajtra/pozajtra".
5. Over, že **Listings, Sales, Finance stránka a zvyšok Event Workspace vyzerajú a fungujú presne ako pred
   týmto releasom** — nič z toho by touto zmenou nemalo byť ovplyvnené.
6. Ak ti "podľa tier" chýba naozaj, daj vedieť — najmenšia oprava je popísaná v bode 5 vyššie, je to ale
   reálna nová databázová zmena, nie rýchla úprava.
