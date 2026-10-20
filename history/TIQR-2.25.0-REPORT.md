# TIQR 2.25.0 — sedem vecí, ktoré si pýtal

---

## 1. Insights a Support sú úplne na konci

Presunuté na koniec zoznamu sekcií v Settings. Jedna zmena poradia pokrýva aj
úvodný zoznam aj bočnú lištu sekcií — obe čítajú ten istý zoznam.

---

## 2. Recap je teraz príbeh

Otvorí sa ako **sekvencia**, nie ako report. Titulok („This is your monthly
recap"), efekt, jedna časť, ďalší efekt, ďalšia časť — a na konci karta, ktorá
ťa pustí do plného reportu. **Report sa nezmenil ani o riadok**, len sa k nemu
teraz prichádza cez príbeh. „Skip to the full report" funguje kedykoľvek,
„Replay" v hlavičke spustí príbeh znova.

Čo sa ukazuje, v poradí: koľko si kúpil → koľko predal → **koľko si zarobil**
(ROI, marža, na lístok) → najlepší kanál → najlepší event → čo ti ešte zostáva
v ruke. Pri Finance recape: money in → money out → zisk → čo ti ešte dlžia →
čo sedí v sklade.

**Ani jedno nové číslo.** Každý slide číta pole, ktoré report už zobrazuje, z
toho istého jedného volania `get_dashboard` — takže slide a report si nikdy
nemôžu protirečiť. **Slide, ktorý nemá čo pravdivé povedať, sa nepostaví
vôbec**: prázdne obdobie to povie priamo, a dve čísla, ktoré appka naozaj
nedrží (koľko dlžíš na nezaplatených objednávkach, zisk na event za obdobie),
to majú napísané na slide namiesto vyplnenia.

Ovládanie: sám sa posúva po 4,2 s, šípky preskakujú, medzerník pauzuje, klik
vľavo/vpravo krokuje. Posledný slide sa neposúva sám — tam sa rozhoduješ.

---

## 3. All time — a je to predvolené

Toto bola tá tvoja poznámka: *„ked som chcel aby mi vsetko ukazalo tak som
musel dat last 6 months. a realne pracujem na tiqr mesiac max dva."*

Pribudlo **All time** ako prvá voľba a **Recap sa na nej otvára**. S jedným
až dvoma mesiacmi histórie sa predtým otváral na skoro prázdnom mesiaci a
jediný preset, ktorý ukázal celý biznis, bolo šesťmesačné okno — čo je zlá
otázka na kladenie.

**Žiadna zmena backendu.** `period_bounds` už mala vetvu `all`; All time vracia
presne tie isté dva dátumy, ktoré tá vetva produkuje. A `previous_period_bounds`
ich už predtým rozpoznávala a vracia `None` — preto porovnávacia tabuľka
správne povie, že nie je s čím porovnávať, namiesto vymysleného okna pred
rokom 1.

---

## 4. Guide ťa teraz naozaj prevedie appkou

Tých sedem odstavcov je preč. Namiesto nich je **14-krokový sprievodca**, ktorý
**naviguje na skutočné stránky**, nájde skutočný prvok, stmaví všetko ostatné a
vysvetlenie položí vedľa toho, o čom hovorí.

Prejde: bočné menu → **hlavné kolónky na dashboarde** → prepínač obdobia →
Events → Orders → Tickets → Sales → Inventory → Pulls → Finance → Price Checker
→ Calendar → sync a zálohy → Recap.

Prvky sa prihlasujú atribútom `data-tour`. Sú v **štyroch súboroch** a každý je
holý atribút bez logiky — `PageHeader` v `ui.tsx` (jedna úprava dala kotvu
každej stránke v appke), bočné menu v `Layout.tsx`, dve na dashboarde.

**Krok, ktorého kotva na obrazovke nie je** (stránka sa ešte načítava, skupina
Tickets je zabalená, stránka nemá tlačidlá), **aj tak zbehne** — len
vycentrovaný a bez reflektora. Sprievodca nikdy neukazuje na nič a nikdy
nečaká na prvok, ktorý nepríde.

---

## 5. Restore zoznamy sa zavrú, keď odídeš

Ten zoznam na tvojom screenshote — „Earlier versions in Google Drive" — **nemal
žiadne zbalenie**: načítal všetky verzie, čo Google drží, a nechal ich tam.

Teraz ukazuje dve s „Show N more", oba zoznamy majú **Hide**, a **odchod zo
sekcie Data ich vráti tam, kde boli** — zbalené, Drive zoznam ani nenačítaný.
Prepínanie sekcií v Settings tú stránku neodmontuje, preto je to naviazané na
sekciu a nie ponechané na unmount.

---

## 6. CSV — tri stĺpce, ktoré v databáze boli a do žiadneho exportu sa nedostali

- **Orders** dostalo `event_date` a `external_reference`. Referencia je na
  objednávkach od migrácie 009; dátum eventu je ten, ktorý si v 2.23.0 nechal
  dať do zoznamu Orders ako viditeľný dátum — a export stále niesol len
  `purchase_date`.
- **Tickets / Inventory** dostalo `event_date`.
- **Sales** dostalo `event_date`.

Všetko **pripojené na koniec**, takže každý stĺpec, ktorý tvoj existujúci
hárok alebo skript už číta, ostáva na svojom mieste. **TBD event exportuje
prázdny dátum** — nikdy vymyslený.

**Skontrolované a zámerne nezmenené:** Events CSV je kompletné — jeho stĺpec
`category` vyzerá ako starý legacy text, ale `resolve_category_name` ho drží v
súlade s `category_id`, takže je správny. Overil som to, nie odhadol.

**Čo NEEXISTUJE a ty povedz, či to chceš:**
- **Pulls nemajú export vôbec** — buyer, event, dátum, počet, platforma, sedadlá,
  cena, deadline prevodu. Nikde.
- **Finance nemá export vôbec** — účty, transakcie, opakované náklady.
- **Ticket listings** (marketplace, listing ID, URL, cena, stav) nemajú export.
- **Sales stále nenesie `batch_id`** — z CSV sa nedá zrekonštruovať, ktoré
  riadky patrili do jedného predaja. `batch_id` je chránený, takže som sa ho
  nedotkol namiesto toho, aby som ho pri príležitosti zmenil.

To sú štyri nové exporty, nie chýbajúce stĺpce — preto som ich nestaval sám.

---

## 7. K návrhom sa dá pridať fotka

Jeden voliteľný obrázok. **Zmenší sa na canvase ešte pred odoslaním** — žiadna
knižnica na obrázky, žiadne Firebase Storage, žiadna nová závislosť. 12 MP
fotka z telefónu nie je to, čo opustí stroj.

Ukladá sa priamo do dokumentu návrhu, preto je strop 600k base64 znakov:
Firestore povoľuje 1 MiB **na dokument** a text s metadátami ho zdieľajú s
obrázkom. Kvalita klesá po krokoch, kým sa nezmestí, namiesto odmietnutia
veľkej fotky. V admin schránke sa obrázok zobrazí, a do `<img>` sa nikdy
nedostane nič iné než `data:image/` URL.

**`firestore.rules` treba znova vložiť do Console** — nové pravidlo obmedzuje
to pole aj na strane servera, lebo pravidlo, ktoré verí appke, že obrázok
zmenšila, nie je pravidlo.

---

## 8. Čo som naozaj otestoval

**Spustené tu:**
- **Príbeh vykreslený v prehliadači** — s presnými Tailwind triedami a presnými
  keyframes z `index.css`. Skontrolované: otvárací slide, ziskový slide s tromi
  číslami a poznámkou, slide s dlhým názvom eventu (63 znakov → zlomí sa na dva
  riadky a zmestí sa), a to všetko aj na okne **820×500**, nie len na celej
  obrazovke.
- **Sprievodca vykreslený v prehliadači** — reflektor na bočnom menu, na
  hlavných kolónkach dashboardu, na tlačidlách v hlavičke, a krok s **neexistujúcou
  kotvou**, ktorý sa správne vycentruje bez pokazeného reflektora.
- **Všetky tri nové CSV SELECTy spustené proti skutočnej SQLite** so
  zaseknutými dátami — vrátane TBD eventu, ktorý naozaj vyexportoval prázdno.
- **Počty stĺpcov hlavičky vs. riadku** pre všetky štyri exporty sedia
  (18/18, 20/20, 27/27, 12/12). Pri nesúlade `csv` crate padne až za behu, nie
  pri kompilácii — preto to bolo treba skontrolovať.
- **JSX a zátvorky** vo všetkých zmenených súboroch proti predchádzajúcemu
  buildu, stack-based, nie počítaním znakov.

**Napísané, spustí sa v CI:** 3 nové testy na CSV export (event_date a
external_reference dorazia; TBD event ostane prázdny; existujúce stĺpce sa
neposunuli).

**Nespustené tu:** `npx tsc -b`, `npm run build`, `cargo check --lib`,
`cargo test --lib` — na tomto Macu nie je Rust ani Node, a nedá sa to sem
dostať bez sťahovania. Toto je to isté obmedzenie, ktoré nechalo dve chyby z
2.18.0 prežiť šesť verzií.

---

## 9. Limity — čo NEsľubujem

- **Príbeh nie je nový report.** Nezobrazí nič, čo report nemá. Ak chceš v ňom
  číslo, ktoré appka nepočíta, treba najprv ten výpočet — a to je iná úloha.
- **Sprievodca vysvetľuje, nič nerobí.** Nič neklikne za teba a nič nezmení.
- **Krok bez kotvy je vycentrovaná karta**, nie chyba. Ak stránka pri tvojom
  behu ešte načítava, ten krok stratí reflektor — text ostane.
- **Fotka je jedna a je v dokumente.** Nie galéria, nie príloha ľubovoľnej
  veľkosti. Nad 600k znakov po kompresii to odmietne a povie prečo.
- **Pulls, Finance a listings stále nemajú CSV export.** Vyššie je to napísané
  ako otázka pre teba, nie ako niečo, čo som ticho preskočil.
