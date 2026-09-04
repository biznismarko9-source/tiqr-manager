# TIQR Manager 2.4.1 — Price Checker Live Market Monitor

Toto je report k tomu, čo si žiadal namiesto "Live Event Intelligence" —
tvoje vlastné slová boli *"Predchádzajúci nápad 'Live Event Intelligence'
RUŠÍME ÚPLNE"*. Ten smer som po tvojom rozhodnutí odstránil úplne (tabuľka,
migrácia, celý backendový modul, TypeScript typy aj UI blok na Event
Workspace) — v appke dnes z toho nie je nič. Report k tomu zrušenému smeru
som premenoval na `REDESIGN-2.4.0-LIVE-EVENT-INTELLIGENCE-REVERTED-REPORT.md`
a nechal len ako históriu, keby sa niečo podobné niekedy hodilo ako
referencia. Tento súbor (`REDESIGN-2.4.1-REPORT.md`) je od teraz ten
skutočný — popisuje, čo appka **reálne robí** vo verzii 2.4.1.

**Prečo 2.4.1, a nie 2.4.0.** Prvý balíček tejto funkcie som pôvodne označil
znova ako "2.4.0" (rovnaké číslo, aké malo aj zrušené "Live Event
Intelligence"), keďže ten smer sa nikdy reálne nevydal cez
`1-CLICK-UPDATE.bat`/GitHub — dostal si ho len ako balíček na prezretie,
takže žiadna inštalácia nikde nemá zaznamenané "2.4.0" a opätovné použitie
čísla samo o sebe bezpečné bolo. Problém bol praktický, nie s auto-updaterom:
súbory s rovnakým názvom (`REDESIGN-2.4.0-REPORT.md`,
`tiqr-manager-2.4.0.zip`) si už raz dostal skôr v tomto rozhovore pre ten
zrušený smer, takže druhé doručenie pod identickým názvom sa nedalo poriadne
stiahnuť. Preto appka aj všetky súbory teraz nesú **2.4.1** — jednoznačne
odlíšené od oboch predošlých "2.4.0" príloh. Číslo migrácie "026" zostáva
znovu použité (o tom nižšie) — to je nezávislé rozhodnutie, lebo migrácie
nemajú s auto-updaterom ani so sťahovaním súborov nič spoločné. Celé
odôvodnenie je zapísané v `PROJECT_STATE/PROTECTED_AREAS.md` pre prípad, že
sa niečo podobné stane nabudúce.

Predtým, než som čokoľvek písal, prešiel som si znova (nie zo starej pamäte)
`price_checker.rs`, `price_checker_scanner.rs` a `price_checker_analysis.rs`
— presne podľa tvojho pokynu "nerob full repo scan, ale over si čerstvo, čo
už existuje, než pridáš niečo nové".

## Ako to funguje: EVENT → ZDROJE → SCAN → SNAPSHOT → HISTÓRIA → ZMENY → ALERTY

Celá nová funkcia stojí na tom, čo už appka mala — **Visible Scanner**
(2.1.9, skutočné viditeľné okno, ktoré si sám scrolluješ a appka prečíta len
to, čo je práve na obrazovke) a **Market Analysis** (2.2.0, rozdelenie
ponuky podľa Tier/Level). Nič z toho som neprepisoval, len som na to naviazal
ďalšiu vrstvu:

- **Marketplace zdroje** zostávajú presne tri — Viagogo, Vivid Seats,
  Ticombo. Žiadny iný marketplace som nepridával.
- **Každý úspešný alebo čiastočný scan teraz automaticky uloží snapshot** —
  trvalý záznam (nikdy sa neprepíše, nikdy sa nezmaže), s celkovými číslami
  aj rozpisom podľa Tier/Level. Sekcia/rad/sedadlo sú aj tu len popisné
  údaje — nikdy neovplyvňujú cenu ani zoskupenie.
- **Market History** — tlačidlo "History" na každej karte marketplace-u
  ukáže posledných 30 snapshotov s dátumom, výsledkom scanu a číslami
  (najnižšia/medián/priemer/najvyššia cena, počet ponúk).
- **Zmeny sa porovnávajú automaticky** po každom úspešnom scane — nový
  snapshot oproti tomu predchádzajúcemu, celkovo aj po jednotlivých
  Tier/Level skupinách zvlášť. Prahové hodnoty sú rovnaké, aké appka už
  používa inde (5 % pre cenu — rovnaké ako "Recommended price"; 20 % pre
  počet ponúk — rovnaké ako Inventory Intelligence) — nič nové som
  nevymýšľal, len znovu použil to, čo si už appka overila.
- **Alerty**: MARKET DROP, MARKET RISE, NEW SUPPLY, SUPPLY DROP, SOURCE
  FAILURE. Posledné — SOURCE FAILURE — je zámerne "tichý": ozve sa len
  vtedy, keď marketplace **prejde** z fungujúceho stavu na zlyhaný, nikdy pri
  úplne prvom pokuse (ten ešte nič nevie porovnať) a nikdy opakovane pri
  viacerých zlyhaniach za sebou. Inak by appka spamovala rovnaký alert
  znova a znova.

## Auto Monitor a "Scan All" — žiadna nová automatizácia navyše

Toto je najdôležitejšia hranica z tvojho zadania a chcem ju zdôrazniť: **nič
tu nikdy neobchádza CAPTCHA, nerotuje proxy, ani neobchádza žiadnu
anti-bot ochranu.** Auto Monitor je len časovač na už otvorenom okne, ktoré
si sám otvoril — v presne zvolenom intervale (15 min / 30 min / 1 hod / 3
hod / 6 hod) zavolá presne to isté "Scan Visible Prices", čo by si spravil
kliknutím sám. Nikdy neotvára okno samo, nikdy nenaviguje, nikdy nečíta nič
iné, než čo by prečítalo tvoje vlastné kliknutie. Keď okno zavrieš, Auto
Monitor sa sám vypne — nezostane "zapnutý" na niečom, čo už neexistuje.
"Scan All" robí to isté naraz pre všetky marketplace karty daného eventu,
ktoré už majú otvorené okno — marketplace bez otvoreného okna jednoducho
preskočí (nič mu neotvorí sám).

## Karta marketplace-u — čo pribudlo

Každá karta (Viagogo/Vivid Seats/Ticombo) dostala nový blok "Live Market
Monitor":
- stav zdroja (Not connected / Connected / Success / Failed),
- kedy bol posledný **úspešný** scan — toto číslo sa nikdy nevymaže ani
  neskryje kvôli neskoršiemu zlyhaniu (appka zostáva plne použiteľná aj
  úplne offline, na posledných uložených dátach),
- krátka, čitateľná chybová hláška pri zlyhaní (nikdy technický výpis),
- čísla z posledného snapshotu,
- ovládanie Auto Monitor + interval,
- tlačidlo History,
- posledné Market Alerty pre túto kartu.

Nič z pôvodnej funkčnosti Price Checkera (Visible Scanner, Market Analysis,
ručné ukladanie do histórie) som sa nedotkol — všetko pôvodné funguje presne
tak ako predtým.

## Dashboard Attention Center — 6. box, "LIVE MARKET ALERTS"

Tvoje zadanie hovorilo o boxe "MARKET ATTENTION" v Attention Centri — narazil
som ale na to, že **tento presný názov appka už používa** (box
"MARKET ATTENTION" z 2.2.11, čo je úplne iná vec: porovnáva TVOJE vlastné
ceny s trhom, nie živé zmeny z Live Market Monitora). Aby sa tieto dve veci
nikdy nepomiešali, nový box som pomenoval **"LIVE MARKET ALERTS"** — je to
môj vlastný rozhodnutý krok, píšem ho sem nahlas, keby si chcel iný názov.

Box ukazuje najnovší alert pre každú dvojicu event/marketplace. Klik naň ťa
zoberie rovno na Price Checker, na presne ten event a marketplace (appka
tam za teba scrollne a na chvíľu kartu zvýrazní) — žiadny nový samostatný
dashboard som nevytváral, presne ako si žiadal. Tento box je jediná zmena v
Attention Centri — samotný výpočet zvyšných 5 boxov som sa nedotkol vôbec.

## Spoľahlivosť a offline správanie

- Jeden pokazený marketplace nikdy neblokuje ostatné — každý scan aj každé
  ukladanie snapshotu/alertu beží nezávisle.
- appka nikdy nezamrzne — Auto Monitor aj Scan All len plánujú tie isté
  volania, čo beží aj dnes, žiadne nekonečné opakovanie.
- Zlyhanie nikdy nezmaže staré dáta — posledný úspešný snapshot a jeho
  čísla ostávajú viditeľné, kým nepríde ďalší skutočný úspech.

## Rozhodnutia, ktoré som urobil sám (over si ich, ak si to myslel inak)

1. **Názov "LIVE MARKET ALERTS" namiesto "MARKET ATTENTION"** — vysvetlené
   vyššie, jediný dôvod je kolízia s existujúcim boxom.
2. **Priorita alertov v Attention Centri**: SOURCE FAILURE a MARKET DROP
   som označil ako "Attention" (niečo sa zmenilo alebo treba opraviť);
   MARKET RISE, NEW SUPPLY a SUPPLY DROP ako "Info" (len trhové
   pozorovanie, nie pokyn na akciu). Zadanie presne nešpecifikovalo, ktorý
   alert do ktorej skupiny patrí, tak som to odhadol podľa toho, čo appka
   robí aj pri ostatných kategóriách.
3. **Číslo verzie skončilo na 2.4.1**, nie na pôvodne plánovanom znovu
   použitom 2.4.0 — dôvod je čisto praktický (kolízia názvov súborov v tomto
   rozhovore), podrobne vysvetlené vyššie a v `PROTECTED_AREAS.md`.

Všetko ostatné je presne podľa tvojich 20 bodov zo zadania (žiadny CAPTCHA
bypass, Tier/Level ako jediné povinné zoskupenie, Section/Row/Seat nikdy
ako cenový faktor, samostatné cache/offline správanie, forward-only
migrácie, atď.).

## Čo som NEROBIL (zámerne, podľa tvojho zadania)

- Žiadny nový marketplace nad rámec Viagogo/Vivid Seats/Ticombo.
- Žiadne automatické precenenie — appka len upozorní, nikdy sama nezmení
  cenu žiadneho listingu.
- Žiadna nová automatizácia nad rámec čítania už otvoreného okna — žiadny
  headless scraping, žiadna nová sieťová komunikácia s marketplacom.
- Žiadny nový samostatný dashboard — všetko je súčasťou existujúceho Price
  Checkera a existujúceho Attention Centra.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/026_price_checker_market_monitor.sql` — nová
  migrácia (4 tabuľky: `market_snapshots`, `market_snapshot_tiers`,
  `market_source_status`, `market_alerts`).
- `src-tauri/src/commands/price_checker_monitor.rs` — nový modul (detekcia
  zmien, snapshoty, alerty, 2 nové príkazy).
- `src-tauri/src/commands/price_checker_scanner.rs` — 2 nové háčiky (po
  úspešnom aj po zlyhanom scane sa zavolá zápis do Live Market Monitora,
  nikdy neovplyvní to, čo vidíš v okne scanneru).
- `src-tauri/src/commands/price_checker_analysis.rs` — jedna funkcia
  (`partition_by_currency`) sprístupnená na opätovné použitie, žiadna
  zmena správania.
- `src-tauri/src/commands/attention_center.rs` — nová 6. kategória
  (`market_alert`), číta len už hotové alerty, žiadna nová výpočtová
  logika.
- `src-tauri/src/models.rs` — nové DTO štruktúry pre Live Market Monitor +
  2 nové polia na `AttentionCenterItem`.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia
  nového modulu a 2 nových príkazov.

**Frontend:**
- `src/pages/PriceChecker.tsx` — Live Market Monitor panel na každej
  karte, Auto Monitor, Market History modal, tlačidlo "Scan All",
  scroll+zvýraznenie karty pri príchode z Attention Centra.
- `src/pages/Dashboard.tsx` — 6. box "LIVE MARKET ALERTS" v Attention
  Centri, mriežka rozšírená na 6 stĺpcov, klik smeruje na Price Checker.
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania pre Live
  Market Monitor.

**Vydávacie súbory (verzia 2.4.1 na všetkých 9 miestach/7 súboroch):**
- `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
  `src-tauri/Cargo.lock` (regenerovaný cez `cargo check`),
  `package-lock.json` (regenerovaný cez `npm install --package-lock-only`).
- `release.ps1` — `$Version` na "v2.4.1", `$CommitMsg` prepísaný na popis
  skutočnej funkcie.
- `1-CLICK-UPDATE.bat` — titulok aj echo text na "v2.4.1".

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`,
  `CHANGELOG.md` — prepísané tak, aby "2.4.1" jednoznačne znamenalo Live
  Market Monitor, so zachovanou históriou zrušeného smeru aj krátkej
  medzizastávky na "2.4.0".
- Starý report premenovaný na
  `REDESIGN-2.4.0-LIVE-EVENT-INTELLIGENCE-REVERTED-REPORT.md`.

## Čo som overil

```
cargo test --lib   -> 1058 passed, 0 failed, 3 ignored (32 nových testov:
                       27 v price_checker_monitor.rs, 5 v attention_center.rs)
npx tsc -b         -> 0 chýb
npm run build      -> OK (vite build prešiel bez chyby)
```

Testy pokrývajú: prahové hodnoty pre cenu aj počet ponúk (vrátane hraničných
prípadov), zoskupenie podľa Tier/Level (opätovné použitie, nie nová logika),
že Section/Row/Seat nikdy neovplyvní žiadny výpočet, viacero snapshotov v
histórii, "tichý" SOURCE FAILURE alert (prvé zlyhanie nikdy nealertuje,
opakované zlyhania tiež nie), že zlyhanie nikdy nezmaže posledný úspech,
nezávislosť jednotlivých marketplace-ov od seba, a že Attention Center
zobrazí presne jeden alert na dvojicu event/marketplace bez kolízie kľúčov
medzi dvoma marketplacmi na tom istom evente.

Manuálne som si prešiel appku so scannerom otvoreným na testovacích dátach —
Auto Monitor sa zapína/vypína správne podľa otvoreného okna, "Scan All"
preskočí marketplace bez okna, History modal ukáže históriu snapshotov, a
klik na "LIVE MARKET ALERTS" riadok v Attention Centri ma spoľahlivo dostal
na správny event aj marketplace kartu.

---

Toľko k 2.4.1. Dve veci, ktoré som rozhodol sám (názov "LIVE MARKET ALERTS"
a priorita jednotlivých typov alertov v Attention Centri) sú vypísané
vyššie — daj vedieť, ak by si to chcel inak. Všetko ostatné je presne podľa
tvojho zadania, vrátane tvojho "STOP, žiadne ďalšie features" na konci —
nič som nepridával nad rámec toho, čo si opísal.
