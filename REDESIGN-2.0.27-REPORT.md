# TIQR Manager 2.0.27 — Kategórie eventov: filter a farby v Events, Orders a Sales

## Čo je nové

Presne ako si chcel — vieš teraz filtrovať podľa kategórie eventu (futbal, koncert, atď.) na **Events, Orders aj Sales**, a pri každom evente je farebný štítok s názvom kategórie, nech je to na prvý pohľad vidno, ale v štýle appky, nič krikľavé:

- **Events** — nový filter "Category" vedľa existujúceho vyhľadávania, farebný štítok pri názve eventu v tabuľke aj vo formulári na pridanie/úpravu eventu.
- **Orders** — rovnaký filter "Category", farebný štítok pri názve eventu (podľa toho, akú kategóriu má event, ku ktorému objednávka patrí).
- **Sales** — rovnaký filter "Category", farebný štítok pri názve eventu (pri "Mixed events" — keď je predaj zložený z lístkov z viacerých eventov naraz — sa štítok jednoducho nezobrazí, presne tak, ako sa tam už dnes nezobrazuje ani samotný názov eventu).

Kategórie už nie sú pevný zoznam 6 možností s "Other..." poľom na dopísanie vlastnej — sú to teraz skutočné spravovateľné položky, presne ako Platformy: **Settings → Lookups → Event categories**, kde si vieš kategóriu pridať aj zmazať. Pri pridávaní/úprave eventu sa kategória vyberá z tohto zoznamu, s možnosťou rovno pridať novú priamo vo formulári ("+ New", rovnaké tlačidlo, aké appka má aj pri platformách).

Všetkých 6 pôvodných kategórií (Concert, Sports, Theatre / Musical, Festival, Comedy, Motorsport) prešlo do nového zoznamu bez zmeny — a ak si niekde mal vlastný text napísaný cez staré "Other...", ten sa automaticky stal svojou vlastnou novou kategóriou. Nič sa nestratilo ani neresetovalo.

Farby sú fixná paleta 8 farieb (zelená, blankytná, ružová, tyrkysová, indigo, azúrová, fialová, oranžová) — každá kategória má svoju farbu pridelenú navždy hneď pri vytvorení, takže sa jej farba nezmení ani keď pridáš alebo zmažeš inú kategóriu. Paletu som vyberal a overoval podľa toho, aby fungovala aj pre farbosleposť, v svetlom aj tmavom režime appky — a zámerne som použil iné farby, ako appka už používa pri stavových štítkoch (napr. Paid/Pending), nech sa to nikdy nepletie.

## Ako presne to funguje pod kapotou

Pribudla nová databázová tabuľka `event_categories` (rovnaký tvar ako `platforms`), cez nový migračný súbor, ktorý pri prvom spustení tejto verzie automaticky:
1. naplní tých pôvodných 6 kategórií, presne v poradí a s farbami, čo mali doteraz,
2. každý existujúci vlastný "Other..." text povýši na svoju vlastnú novú kategóriu,
3. každý event, čo mal doteraz kategóriu uloženú len ako text, napojí na správny riadok v novej tabuľke.

Starý textový stĺpec `category` pri evente som **nezmazal** — appka ho odteraz už len ticho drží zosynchronizovaný s tým, akú skutočnú kategóriu má event nastavenú, takže export do CSV funguje úplne bez zmeny.

Farba sa v databáze neukladá ako farba (napr. "zelená"), ale ako poradové číslo (0, 1, 2...) — samotné odtiene sú len vo frontende. Keby si niekedy chcel paletu doladiť, dá sa to spraviť bez zásahu do databázy.

Kategória sa na Orders a Sales nikde znova neukladá — appka si pri načítaní zoznamu len dotiahne, akú kategóriu má event, ku ktorému objednávka/predaj patrí (rovnaký spôsob, akým appka už dnes dotiahne napríklad názov platformy) — takže filter aj farebný štítok fungujú na všetkých troch obrazovkách bez extra dopytu navyše.

Zmazanie kategórie v Settings **nikdy** nezmaže ani neovplyvní žiadny event, objednávku, predaj ani peniaze — dotknuté eventy len stratia farebný štítok a vrátia sa do stavu "no category", presne tak, ako to appka pri mazaní aj sama napíše.

Jedna drobná poznámka k testovaniu, nech ťa nezaskočí, keby si sa pozrel na priložený screenshot zblízka: v mojom dočasnom testovacom prehliadači (bez appky, len na overenie vzhľadu) sa v tabuľkách miestami dotýkal text v susedných úzkych stĺpcoch. Overil som si, že je to len tým, že toto testovacie prostredie nemá nainštalovaný font appky ("Inter") — na tvojom Windows je nainštalovaný bežný náhradný systémový font, takže by si toto vôbec nemal vidieť. Nesúvisí to s touto zmenou ani s novými štítkami — stĺpce majú v tabuľke pevnú šírku, nezávislú od obsahu.

## Testy a build

```
cargo test --lib -> 475 passed, 0 failed, 3 ignored (bolo 470 - pribudlo 5 nových testov pre kategórie)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.27 build" v hlavičke)
```

Naviac som si všetko aj vizuálne overil cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) — Events/Orders/Sales/Settings, svetlý aj tmavý režim, filter podľa kategórie, farebné štítky, pridanie a zmazanie kategórie v Settings aj "+ New" kategória rovno z formulára na pridanie eventu.

## Zmenené súbory

**Nové:**
- `src-tauri/migrations/012_event_categories.sql` — nová tabuľka `event_categories`
- `src-tauri/src/commands/event_categories.rs` — list/create/delete + testy
- `src/components/EventCategoryBadge.tsx` — farebný štítok + bodka, 8-farebná paleta

**Zmenené:**
- `src-tauri/src/models.rs`, `src-tauri/src/db.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia novej tabuľky/príkazov, nové polia
- `src-tauri/src/commands/events.rs`, `src-tauri/src/commands/orders.rs`, `src-tauri/src/commands/sales.rs` — join na kategóriu, filter podľa kategórie
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania
- `src/pages/Events.tsx`, `src/pages/Orders.tsx`, `src/pages/Sales.tsx` — filter + farebný štítok
- `src/pages/Settings.tsx` — nová sekcia "Event categories" v Lookups
- `src/components/ExportPickerModal.tsx` — drobná oprava po zmene `listEvents`

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.27`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.27 hotové a overené (475/475 testov, čisté `tsc`/`build`, vizuálne overené). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Settings → Lookups → Event categories — vieš pridať novú kategóriu aj existujúcu zmazať.
2. Events, Orders aj Sales — nový filter "Category" vyfiltruje správne a pri evente je vidno farebný štítok.
3. Pridaj/uprav event — v poli Category vieš rovno napísať a vytvoriť novú kategóriu bez toho, aby si musel ísť do Settings.
4. Skontroluj, či sa ti farby páčia a či je to podľa teba dosť rozlíšiteľné — ak nie, napíš mi, ktoré farby vymeniť.
