> **POZNÁMKA (pridané dodatočne):** Tento report opisuje smer "Live Event
> Intelligence", ktorý marko po prezretí tohto reportu úplne zrušil
> ("Predchádzajúci nápad 'Live Event Intelligence' RUŠÍME ÚPLNE") skôr, než
> bol kedy reálne vydaný — v appke dnes neexistuje nič z toho, čo je nižšie
> opísané (tabuľka, migrácia, modul, UI). Súbor je premenovaný a ponechaný
> len ako história/referencia pre prípad, že podobná funkcia bude niekedy
> nanovo navrhnutá. Číslo "2.4.0" bolo krátko aj číslom tejto nahrádzajúcej
> funkcie, kým sa pri doručovaní súborov v tomto rozhovore neukázalo, že
> rovnaké názvy súborov (`REDESIGN-2.4.0-REPORT.md`,
> `tiqr-manager-2.4.0.zip`) už raz použité pre TENTO zrušený smer robia
> súbor nesťahovateľný — appka aj všetky súbory preto dnes reálne nesú
> **2.4.1**. Pozri `REDESIGN-2.4.1-REPORT.md` (Price Checker Live Market
> Monitor) pre to, čo v appke reálne je.

# TIQR Manager 2.4.0 — Live Event Intelligence Foundation (ZRUŠENÉ, nikdy nevydané)

Dostal som tvoj podrobný zadávací list na novú vrstvu "Live Event Intelligence" —
event už nemá byť len lokálny záznam, ale môže mať pripojenú online identitu na presne
3 marketplacoch (Viagogo, Vivid Seats, Ticombo — žiadny iný, StubHub ani Seatriks som
nepridával). Zadanie bolo dlhé a podrobné (online identita eventu, model zdrojov, UI
sekcia, discovery flow, cache/offline správanie, žiadny zásah do Price Checkera, žiadna
cenová logika), takže namiesto prepisovania celého zadania vlastnými slovami rovno
prechádzam bod po bode presne tak, ako si ich na konci sám vypísal.

Predtým než som čokoľvek písal, prešiel som si `CURRENT_STATE.md` a `PROTECTED_AREAS.md`
a priamo v kóde overil Events/EventDetail, Price Checker, existujúci Visible Scanner
(`price_checker_scanner.rs`) a event DB/model vrstvu — presne podľa tvojej inštrukcie
"nerob full repo scan".

## 1. Nová online identita eventu

Event teraz môže mať 0 až 3 pripojené online zdroje (jeden na marketplace, nikdy
viac). Každý zdroj nesie: URL, voliteľné externé ID eventu (ak ho zadáš ručne),
**verified** (overil si ho ty osobne v reálnom okne) a **active** (stále pripojený,
alebo "Disconnect"-nutý), plus kedy a čo appka naposledy videla (`last_checked_at` +
presne to, čo v tú chvíľu ukazoval titulok stránky). Žiadne z toho appka sama od
seba nevymyslela ani nezhádala — každý riadok vznikne len tvojou explicitnou akciou
(Find Online Event → potvrdenie kandidáta, Refresh → potvrdenie, alebo Connect
manually).

**Existujúce dáta eventu (`events` tabuľka) som sa nedotkol vôbec** — nová identita
žije v úplne samostatnej tabuľke, nie ako nové stĺpce na evente. Žiadny existujúci
event, jeho štatistiky ani žiadna existujúca query/test sa touto úlohou nezmenili.

## 2. DB / migrácia

Nová migrácia `026_live_event_intelligence.sql`, nová tabuľka `event_online_sources`:

```sql
CREATE TABLE event_online_sources (
  id, event_id (FK -> events, ON DELETE CASCADE),
  source TEXT CHECK IN ('viagogo','vivid_seats','ticombo'),
  url, external_event_id,
  verified INTEGER DEFAULT 0,
  active INTEGER DEFAULT 1,
  last_checked_at, last_checked_title,
  created_at, updated_at,
  UNIQUE(event_id, source)
);
```

`UNIQUE(event_id, source)` je tvoje pravidlo "marketplace najviac raz na event"
priamo v schéme, nie len v kóde. Migrácia je dopredná (forward-only), nič
neprepisuje ani nedopĺňa zo starých dát — overené aj testom, ktorý simuluje reálny
upgrade z 2.3.5 (25 starých migrácií + reálny event) a kontroluje, že po pridaní
026-ky ostane every existujúci event bajt po bajte nezmenený.

**Dôležité rozhodnutie**: tabuľka NIE JE napojená (foreign key) na existujúcu
tabuľku `marketplaces`, ktorú používa Price Checker/Listings (tam, kde je aj
vyradený StubHub a Seatriks). Je to zámerne úplne oddelený, pevne daný zoznam len
pre túto funkciu — presne podľa tvojho "Podporuj LEN tieto 3, nepridávaj žiadny
iný". Keby som to napojil na tú istú tabuľku, niekto by cez Live Event Intelligence
mohol teoreticky pripojiť aj StubHub. Dôvod aj dôsledky (že zmazanie marketplacu z
Price Checkera túto tabuľku vôbec neovplyvní, a naopak) mám podrobne zapísané v
`PROTECTED_AREAS.md`, nová sekcia "2.4.0".

## 3. Podporované marketplace

Presne 3: **Viagogo, Vivid Seats, Ticombo**. Vynútené na dvoch miestach naraz —
`CHECK` v databáze aj kontrola v Rust kóde (`SUPPORTED_SOURCES`) — takže ani chyba v
kóde, ani priamy zápis do databázy nemôže pretlačiť StubHub/Seatriks/čokoľvek iné
(mám na to aj samostatný test). Keby si niekedy chcel pridať 4. zdroj, je to malé,
prídavné rozšírenie (nová migrácia na CHECK + jeden riadok na frontende s
vyhľadávacou URL) — žiadny zásah do kódu tých troch existujúcich.

## 4. Discovery flow (Find Online Event / Refresh / Connect manually)

Toto je srdce úlohy a najviac som sa sústredil na to, aby to bolo bezpečné presne v
duchu tvojho Visible Scanneru (2.1.9) — **žiadne API, žiadne scrapovanie, žiadny
CAPTCHA/anti-bot bypass, nič sa nevypĺňa ani neodosiela samo**.

- **"Find Online Event"** otvorí skutočné, VIDITEĽNÉ okno prehliadača (rovnaká
  technika ako Visible Scanner — reálne Tauri okno na vlastnom vlákne, aby sa
  appka nezasekla) na predvyplnenej vyhľadávacej URL pre zvolený marketplace
  (postavenej z názvu eventu + mesta). Ty si v tom okne hľadáš úplne sám, presne
  ako v bežnom prehliadači — appka do vyhľadávania vôbec nezasahuje.
- **"Capture this page"** prečíta LEN to, čo je práve na obrazovke: titulok
  stránky a jej URL. Nič viac — žiadne ceny, žiadne listingy, žiadne parsovanie
  obsahu. Každé kliknutie pridá jedného "kandidáta" do zoznamu.
- **"Use this one"** pri konkrétnom kandidátovi je JEDINÉ miesto v celej appke,
  ktoré uloží zdroj ako **verified**. Kým nekliknem — nič sa neuloží ako potvrdené.
- **"Refresh"** pri už pripojenom zdroji robí presne tú istú vec, len rovno na
  uloženej URL namiesto nového hľadania — a je to aj spôsob, ako sa ručne pripojený
  (zatiaľ neoverený) zdroj stane overeným, bez potreby osobitného tlačidla "mark as
  verified".
- **"Connect manually"** obíde okno úplne — keď už URL máš, len ju vložíš (a
  voliteľne externé ID). Takto uložený zdroj je vždy **"Not verified"**, kým ho
  raz nepotvrdíš cez Refresh — appka predsa tú stránku sama nikdy nevidela.

Nikde appka sama nevytvára nový event, nikdy sama nevyberie "ten správny" kandidát
a nikdy nezapíše neistý výsledok ako potvrdený — presne tvoje "nikdy neuložiť
neoverené dáta ako potvrdené" a "pri nejasnej zhode uprednostni Needs confirmation".

## 5. Cache / offline správanie

- Appka sa online nespýta NIKDY sama od seba — len explicitným kliknutím na Find
  Online Event alebo Refresh.
- Otváranie okna aj čítanie stránky bežia na samostatnom vlákne s časovým limitom
  (10 sekúnd na jedno čítanie) — presne rovnaký mechanizmus ako Visible Scanner.
  Appka sa pri tom nikdy nezasekne, ani keď stránka vôbec neodpovie.
- Chyba (timeout, zatvorené okno, nečitateľná odpoveď) ukáže krátku, ľudskú
  hlášku (napr. "Stránka neodpovedala včas — skús to znova."), nikdy technický
  výpis.
- Zoznam pripojených zdrojov aj ich posledný stav sa číta výhradne z lokálnej
  databázy — appka funguje úplne bez pripojenia na internet, s poslednými
  uloženými dátami.
- **Appka nikdy sama nekontaktuje Viagogo/Vivid Seats/Ticombo cez žiadne
  pozadové volanie** (napr. ani jednoduchý "je link ešte živý" ping) — jediný
  spôsob, akým appka vôbec príde do kontaktu s týmito stránkami, je otvorenie
  skutočného viditeľného okna, ktoré ovládaš ty. Toto je zámerné rozhodnutie —
  detaily a dôvod (rovnaká hranica, akú sme si stanovili aj pri tvojej otázke na
  auto-listing bota na Ticketmastri) sú v `PROTECTED_AREAS.md`.

## 6. UI zmeny

Na Event Workspace → Overview (nad Inventory Intelligence) pribudla kompaktná
karta **"Live Event Intelligence"** — vždy presne 3 riadky, jeden na marketplace:

- Nepripojený → tlačidlá **Find Online Event** / **Connect manually**.
- Pripojený → značka **Verified**/**Not verified**, URL, "Last checked" (+
  posledný videný titulok stránky), tlačidlá **Refresh** / **Open source**
  (otvorí link v tvojom bežnom prehliadači) / **Disconnect**.
- Odpojený (ale predtým pripojený) → zostáva viditeľný, prečiarknutý, s
  tlačidlom **Reconnect** — nič sa "Disconnectom" nezmaže, len sa prestane počítať
  ako aktívne pripojené.

Žiadny nový veľký tab, žiadny zásah do zvyšku Event Workspace — presne tvoje "nech
to je prirodzená súčasť eventu, nie samostatná appka v appke".

## 7. Čo som overil

```
cargo test --lib   -> 1042 passed, 0 failed, 3 ignored
                      (1023 pôvodných + 19 nových v live_event_intelligence.rs
                      + 3 nové migračné testy v db.rs, žiadna zmena správania
                      v pôvodných testoch)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.4.0 build" v hlavičke)
```

Nové testy pokrývajú presne to, čo si žiadal: event bez zdroja, s 1 zdrojom, s
viacerými zdrojmi naraz, zabránenie duplicite marketplacu, ručné pripojenie,
potvrdenie kandidáta, neoverený kandidát, refresh (úspech aj to, že neoverený
zdroj sa refreshom stane overeným), offline/cache čítanie, správne ukladanie
last_checked, existujúce eventy prežijúce migráciu (bajt po bajte), CASCADE
zmazanie, a že StubHub/Seatriks natvrdo odmieta aj CHECK v databáze aj Rust
kód. Celý pôvodný balík (Events/Orders/Tickets/Listings/Sales/Finance/
Fulfillment/Attention Center/Price Checker/jeho scanner) prešiel bez jedinej
zmeny správania — nič z toho som sa nedotkol.

## 8. Čo som zámerne NEspravil

- Žiadne skutočné API ani scraping voči Viagogo/Vivid Seats/Ticombo — jediný
  kontakt s nimi je otvorenie reálneho okna, ktoré ovládaš ty.
- Žiadny automatický výber kandidáta ani automatické vytvorenie eventu z
  webového výsledku.
- Vyhľadávacie URL pre "Find Online Event" som si overil, ako viem, ale
  marketplace môžu svoje vyhľadávanie kedykoľvek zmeniť bez upozornenia — keby
  URL netrafila rovno na výsledky, okno je skutočný prehliadač, takže si
  jednoducho dohľadáš sám, presne ako v bežnej karte prehliadača.
- Žiadna cenová logika, žiadny nový výpočet trhu, žiadne automatické prehodnotenie
  ceny, žiadna cena odvodená zo sekcie/radu — Price Checker ani jeho scanner som
  sa nedotkol vôbec. Section/Row/Seat nikde v tejto funkcii nehrá žiadnu úlohu.
- Žiadny nový veľký tab v Event Workspace.
- StubHub, Seatriks ani žiadny iný marketplace ako zdroj pre Live Event
  Intelligence — natvrdo iba tie 3.

## Zmenené / nové súbory

**Backend:**
- `src-tauri/migrations/026_live_event_intelligence.sql` — nová tabuľka
- `src-tauri/src/db.rs` — registrácia migrácie 026, `LiveIntelSession`,
  `AppState.live_intel_sessions`, 3 nové migračné regresné testy
- `src-tauri/src/models.rs` — `EventOnlineSource` + 3 input typy + 4 typy pre
  udalosti viditeľného okna
- `src-tauri/src/commands/live_event_intelligence.rs` — nový modul (DB
  príkazy + okno/capture príkazy), 19 testov
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia
  nového modulu a 7 nových príkazov
- `src-tauri/src/commands/database.rs` — `AppState` test-helper doplnený o
  nové pole, opravený test počtu migrácií (25 → 26)

**Frontend:**
- `src/lib/types.ts` — `EventOnlineSource` + súvisiace typy,
  `LIVE_EVENT_SOURCES`
- `src/lib/api.ts` — 7 nových wrapperov
- `src/components/icons.tsx` — nová `IconRefresh`
- `src/pages/EventDetail.tsx` — nová karta "Live Event Intelligence" +
  discovery/refresh/manual-connect modály

**Dokumentácia a verzia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`
  (nová sekcia "2.4.0"), `CHANGELOG.md`
- Verzia zvýšená na **2.4.0** vo všetkých 9 miestach (`package.json`,
  `tauri.conf.json`, `Cargo.toml`, `release.ps1` — `$Version` aj
  `$CommitMsg`, `1-CLICK-UPDATE.bat`, `Cargo.lock`, `package-lock.json`)

**Mimochodom** — pri bumpovaní verzie som si všimol, že `release.ps1`-ovo
`$CommitMsg` bolo už predtým zastarané (ešte stále opisovalo 2.2.9-kové opravy,
napriek tomu že `$Version` bolo už 2.3.5) — opravil som ho na presný popis tejto
2.4.0 release, ale stojí za to vedieť, že to tam takto viselo zabudnuté aspoň
jednu celú verziu.

---

To je celé zadanie. Nikde som sa nepýtal na spresnenie — pri "Refresh" (že
nerobí žiadne tiché sieťové volanie, ale rovnaké viditeľné okno ako Find Online
Event) a pri tvare vyhľadávacích URL som urobil rozhodnutie sám, dôvody sú vyššie
aj v `PROTECTED_AREAS.md`. Ak niečo z toho chceš inak, daj vedieť.
