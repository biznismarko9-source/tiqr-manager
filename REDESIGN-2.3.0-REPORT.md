# TIQR Manager 2.3.0 — Event Lifecycle / Event Operations

Toto je ďalší väčší task po 2.2.11/2.2.12 - jedna konzistentná, odvodená
"lifecycle / operational status" fáza pre každý event, presne podľa tvojho
zadania. Žiadny nový stĺpec, ktorý by si musel ručne vypĺňať - fáza sa
vždy počíta znova, zo skutočných dát, ktoré appka už má.

Ako zvyčajne, nikde som sa ťa nepýtal na spresnenie. Tvoje zadanie samo
vyžadovalo pár rozhodnutí - všetky vysvetlené nižšie, s presným dôvodom,
aby si ich vedel opraviť, ak si to myslel inak.

## 1. Lifecycle pravidlá presne

Šesť fáz (nie sedem, ako v tvojom príklade - vysvetlené nižšie), v tomto
poradí, počítané vždy odznova, prvé pravidlo, čo sedí, vyhráva:

```
1. COMPLETED   - event.status je "completed"/"cancelled" ALEBO event_date
                 už prešiel (je striktne v minulosti)
2. EVENT DAY   - event_date === dnešný dátum (celý deň, žiadna hodinová
                 presnosť - appka nemá event time)
3. SELLING     - aspoň jeden predaný tiket
4. LISTED      - nula predaných, aspoň jeden aktívne vystavený tiket
5. INVENTORY   - nula predaných, nula vystavených, aspoň jeden kúpený tiket
6. UPCOMING    - úplne nič kúpené
```

Presne toto je funkcia `computeEventLifecyclePhase` v `src/pages/Events.tsx`
- a je to **čistá funkcia jedného objektu, ktorý appka už aj tak posiela**
(`event.status`, `event.eventDate`, `event.stats.{purchasedTickets,
listedTickets, soldTickets}`) - nič nové sa kvôli tomu nedopočítava, žiadny
nový network/IPC call navyše, ani na zozname Eventov, ani v Event
Workspace.

### Prečo 6 fáz, nie tvojich 7

Navrhol si `UPCOMING → BUYING/INVENTORY → LISTED → SELLING → EVENT DAY →
POST EVENT → COMPLETED`. Dve zmeny:

- **"BUYING / INVENTORY" je tu len "INVENTORY"** - appka už dnes používa
  presne toto slovo (stĺpec "Available" na zozname Eventov = tikety kúpené,
  ale ešte nevystavené) - "buying" by opisovalo prebiehajúcu akciu, ktorú
  appka nevie nijako pozorovať, "inventory" opisuje reálny, pozorovateľný
  stav.
- **POST EVENT som zlúčil do COMPLETED.** Tvoje vlastné pravidlo pre
  COMPLETED je úplne doslovné - "event date passed alebo event status
  completed/cancelled" - a je to presne to isté pravidlo, čo appka už dnes
  volá `isEventDone` (Orders.tsx) / `event_is_done` (backend). Toto
  pravidlo je čisto dátumové/status-ové - to znamená, že **hneď na druhý
  deň po evente je už COMPLETED podľa tvojho vlastného pravidla** - nezostáva
  žiadna reálna medzera, kam by POST EVENT mohol sedieť, bez toho, aby som
  si vymyslel nový, ničím nepodložený "grace period" (koľko dní presne? na
  základe čoho?). Presne toto si sám povolil: "Ak zistíš, že niektorú fázu
  nemožno spoľahlivo rozlíšiť: zlúč ju s najbližšou použiteľnou fázou, uveď
  v reporte prečo."

  **Nič sa tým nestráca** - informácia "eventu sa ešte niečo nedokončilo" aj
  po COMPLETED nezmizla, len nie je súčasťou samotnej fázy - pozri bod 3
  nižšie (Pending fulfillment/Next Actions sa zobrazujú pri KAŽDEJ fáze,
  vrátane COMPLETED).

- **CANCELLED nie je samostatná 8. fáza** - má už dnes svoj vlastný,
  vždy viditeľný Status badge presne vedľa nového lifecycle badge (na
  zozname aj v Event Workspace) - druhý badge, čo by len znova hovoril
  "cancelled", by bol šum, nie nová informácia.

## 2. Zmenené súbory

**Frontend (žiadny backendový `.rs` súbor sa nemenil - potvrdené
`cargo test --lib`, presne rovnaký počet testov ako v 2.2.12):**

- `src/pages/Events.tsx` - nová lifecycle logika (`EventLifecyclePhase`,
  `computeEventLifecyclePhase`, `EVENT_LIFECYCLE_PHASES`,
  `EventLifecyclePhaseBadge`), nový filter, nový riadok v tabuľke
- `src/pages/EventDetail.tsx` - nový blok `EventLifecycleBlock` na
  Overview tabe

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`,
  `CHANGELOG.md`

## 3. UI zmeny

**Zoznam Eventov (`/events`):**
- Pod existujúcim Status badge (upcoming/completed/cancelled) pribudla
  malá "pilulka" s farebnou bodkou + názvom fázy (napr. "● Selling") -
  **žiadny nový stĺpec, žiadna zmena šírky tabuľky.**
- Nový filter "Lifecycle phase" vedľa existujúcich filtrov (Category/
  From/To/Sort) - dá sa vybrať konkrétna fáza alebo "All phases".
  **Kombinuje sa s existujúcimi tabmi Upcoming/Completed (obe platia
  naraz), nenahrádza ich** - tie dve veci sa môžu aj rozísť (event so
  statusom stále "upcoming", ale ktorého dátum už prešiel, sa ukáže s
  fázou COMPLETED, ale stále pod tabom Upcoming) - to je zámerne užitočná
  informácia ("tento event si zabudol označiť"), nie chyba.

**Event Workspace, tab Overview (`/events/:id`):**
Úplne nový blok "Event Lifecycle" hore, nad existujúcimi štatistikami
(Tickets/Sold/Available/...) - presne podľa tvojho príkladu:

```
Event Lifecycle                                    ● SELLING

[progress prúžok - 6 segmentov, aktuálna fáza zvýraznená]

24 tickets · 18 listed · 7 sold · 2 pending fulfillment

NEXT ACTIONS
6 tickets not listed
2 sold tickets not delivered
```

- Progress prúžok ukazuje LEN "kde sme teraz", nie že event prešiel
  postupne každou fázou (cancelnutý event so 0 tiketmi môže preskočiť
  rovno z UPCOMING do COMPLETED).
- Klik na "6 tickets not listed" / "2 sold tickets not delivered" ťa
  prepne na tab Listings / Sales (rovnaký `onSwitchTab` mechanizmus, čo tu
  appka už používa pri Inventory Intelligence).
- Keď nič nepotrebuje pozornosť: "Nothing needs attention for this event
  right now."

## 4. Ako sa status počíta

Fáza samotná (viď bod 1) je 100% odvodená z `event.status`, `event.eventDate`
a `event.stats` - dáta, čo appka aj tak posiela pri `list_events`/`get_event`.
**Žiadna nová databázová tabuľka, žiadna migrácia, žiadny nový backend
príkaz pre samotnú fázu.**

Dve ďalšie čísla v Event Workspace (fázu neurčujú, len sa zobrazujú popri
nej) používajú dva UŽ EXISTUJÚCE príkazy, presne podľa tvojho pokynu
"Použi už existujúce Attention Center a Fulfillment dáta":

- **"Pending fulfillment"** = `list_sale_groups` s jeho existujúcim
  filtrom `eventId` (presne to isté, čo Sales.tsx aj Fulfillment Center už
  volajú), spočítané cez `isSaleGroupDone` (nezmenené, importované zo
  Sales.tsx) - presne Fulfillment Centra vlastná definícia "Pending
  Sales", len obmedzená na jeden event.
- **"Next Actions"** = jeden call na `get_attention_center()` (presne ten
  istý GLOBÁLNY Attention Center, čo vidíš na Dashboarde), prefiltrovaný
  na strane appky len na tento event, zoskupený do jednej vety na
  kategóriu. Backendová logika Attention Center (`attention_center.rs`) sa
  vôbec nemenila - len sa bezpečne znova použila.

Obe čísla sa zobrazujú **pri KAŽDEJ fáze, vrátane COMPLETED** - aj dokončený
event môže mať ešte nedoriešený predaj, a to je presne tak dôležité (ak nie
dôležitejšie) ako pri aktívnom evente. Rovnaký princíp appka už dnes
používa pri "sold_undelivered" v Attention Center.

## 5. Filter

Nový dropdown "Lifecycle phase" na `/events` - vyberieš konkrétnu fázu
(Upcoming/Inventory/Listed/Selling/Event Day/Completed) alebo "All
phases". Funguje spolu s (nie namiesto) existujúcich tabov Upcoming/
Completed a existujúcich filtrov Category/From/To - všetko sa kombinuje
naraz (AND), presne ako doteraz fungovali Category+From+To.

## 6. Test results

```
cargo test --lib   -> 1006 passed, 0 failed, 3 ignored
                      (identické s 2.2.12 - žiadny .rs súbor sa nemenil)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Appka nemá frontend testovací framework (overené - žiadny vitest/jest/
`*.test.*` súbor nikde v histórii projektu), takže frontendová logika sa
overovala rovnako ako doteraz - `tsc -b` + čítanie kódu + úvaha, plus
jednorazový skript (esbuild + Node, mimo repozitára, po overení zmazaný -
nie je súčasťou appky ani ZIPu), ktorý si importoval SKUTOČNÉ funkcie
(`computeEventLifecyclePhase`, `EVENT_LIFECYCLE_PHASES`, `isSaleGroupDone`)
a overil presne tvoje testovacie scenáre:

```
event upcoming bez inventory ....... status upcoming, 0 tiketov -> UPCOMING
event upcoming s inventory .......... kúpené tikety, nič vystavené/predané -> INVENTORY
event s listings ..................... vystavené, nič predané -> LISTED
event so sales ........................ aspoň 1 predaný -> SELLING (aj tvoj presný príklad 24|18|7)
event day .............................. event_date == dnes -> EVENT DAY (má prednosť pred INVENTORY/LISTED/SELLING)
event po dátume ........................ status stále upcoming, dátum včera -> COMPLETED
completed/cancelled event .............. oba statusy -> COMPLETED, bez ohľadu na dátum
precedencia .............................. COMPLETED vyhráva aj keď by tikety hovorili SELLING
filter podľa fázy (4 scenáre) .......... tab a fáza sa správne kombinujú aj rozchádzajú
pending fulfillment ..................... súčet cez isSaleGroupDone na fixture so "všetko pending" prípadmi
Next Actions agregácia .................. súčet ticketIds cez viac zoskupených riadkov tej istej kategórie,
                                           bez miešania s iným eventom

25 z 25 kontrol prešlo.
```

Klik-na-riadok/Next-Actions navigáciu (`onSwitchTab`) som overil čítaním
skutočného kódu (rovnaký mechanizmus, čo appka už používa pri Inventory
Intelligence) - appka tu v tomto prostredí nemá displej, takže naživo cez
skutočné okno appky to nejde spustiť, rovnako ako pri každej predchádzajúcej
frontendovej zmene v tomto projekte.

**Regresie**: Orders/Listings/Sales/Finance/Attention Center/Fulfillment
Center som nemenil vôbec (žiadny z ich súborov nie je v zozname zmenených
súborov vyššie) - jediné dva zmenené súbory (`Events.tsx`, `EventDetail.tsx`)
dostali len PRIDANÝ kód (nová exportovaná funkcia/komponent, nový blok v
JSX), žiadna existujúca funkcia/exportovaný typ nezmenil svoj tvar ani
správanie. `tsc -b` (type-checkuje celý projekt naraz) prešiel na 0 chýb,
čo je silný dôkaz, že nič inde v appke, čo by z týchto súborov importovalo,
sa nerozbilo.

## 7. Limity

- **Progress prúžok neznamená "prešiel každou fázou"** - je to len
  ukazovateľ "kde sme teraz". Event zrušený hneď na začiatku môže preskočiť
  rovno z UPCOMING do COMPLETED.
- **Žiadna hodinová presnosť pre EVENT DAY** - presne podľa tvojho pokynu,
  appka nemá event time, takže sa porovnáva len dátum (celý deň).
- **"Next Actions" je zámerne stručnejší než plný Attention Center** -
  ukazuje jednu vetu na kategóriu (súčet tiketov), nie riadok na každú
  objednávku zvlášť, ako to robí plný Attention Center na Dashboarde. Ak by
  si niekedy chcel aj tu vidieť rozpad po objednávkach, je to malé, čisto
  zobrazovacie rozšírenie - dáta na to appka už má.
- **Lifecycle filter na zozname Eventov sa dá kombinovať s tabmi Upcoming/
  Completed, ale appka ich nenúti súhlasiť** - je to zámer (vysvetlené v
  bode 1 a 5), nie chyba, ale flagujem to explicitne, keby ťa to na prvý
  pohľad prekvapilo.
- **"Pending fulfillment" a Attention Center dáta sa v Event Workspace
  načítavajú samostatným volaním** (rovnaký vzor, čo appka už používa pri
  Inventory Intelligence) - pri veľmi veľkom počte eventov/predajov by to
  teoreticky mohlo byť o čosi pomalšie než keby sa dáta zdieľali, ale
  appka je lokálna (SQLite), takže to v praxi nie je citeľné.

---

Tým je hotový celý task 2.3.0. Jedno miesto na tvoje rozhodnutie: zlúčenie
POST EVENT do COMPLETED (bod 1 vyššie) - ak si to predstavoval inak (napr.
konkrétny počet dní ako "grace period"), daj vedieť, je to jedna funkcia na
zmenu (`computeEventLifecyclePhase` v `Events.tsx`), nič iné by nezasiahla.
Všetko ostatné, vrátane všetkých DÔLEŽITÉ bodov (žiadna zmena refund/
resell/batch_id/money/tier/section/pricing), je presne tak, ako si žiadal.

STOP. Žiadne ďalšie features.
