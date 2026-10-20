# TIQR Manager 2.0.26 — Zmenšené a prehľadnejšie Integrations karty

## Čo je nové

V **Settings → Integrations** (karty Pulls a Orders & Sales) je teraz výrazne menej textu a tlačidlá dávajú zmysel:

- Namiesto 3-5 dlhých odstavcov nad formulárom (jeden pre pripojenie, jeden pre každé tlačidlo) je tam teraz **jedna krátka veta**. Vysvetlenie ku každému tlačidlu nezmizlo — presunulo sa **priamo naň, ako bublinu po podržaní myšou** (rovnaké slovo od slova, nič som nekrátil ani neprepisoval).
- Tlačidlá už nie sú jeden dlhý riadok, čo sa náhodne zalamoval podľa šírky okna. Teraz sú v **3 riadkoch podľa toho, čo reálne robia** — presne ako si to opísal:
  1. Pripojenie: Save/Connect, Test connection, Update sheet, Disconnect (ten je teraz oddelený vpravo, nech nevyzerá ako rovnocenná akcia)
  2. Sem, do appky: Sync now / Order sync + Sales sync
  3. Von, do hárku: Push to sheet / Push orders + Push sales

Karta je vďaka tomu citeľne nižšia — pri bežnej šírke okna sa obe karty (Pulls aj Orders & Sales) zmestia vedľa seba bez toho, aby si musel scrollovať, aby si videl všetky tlačidlá.

## Ako presne to funguje pod kapotou

Nič sa nezmazalo, len presunulo. Predtým bol každý popis (`syncDescription`, `pushDescription`, popisy pri Sales sync/Push sales, `setupDescription`) vlastný odstavec textu, vždy viditeľný. Teraz je to `title` atribút na tlačidle, ktorému patrí — to je štandardný spôsob, ako to robí každá webová stránka (bublina po podržaní myšou), appka na to nepotrebovala žiadnu novú knižnicu ani kód navyše. Overil som priamo v prehliadači, že text v bubline na každom tlačidle (Sync now, Push to sheet, Order sync, Sales sync, Push orders, Push sales, Update sheet) je presne ten istý text, čo tam bol predtým v odstavci — len teraz sa neukazuje stále, len keď ho chceš vidieť.

Toto je čisto frontendová zmena — jeden súbor, `Settings.tsx`, konkrétne komponent `SheetsConnectionCard`, ktorý používajú obe karty (Pulls aj Orders & Sales). Nič v Rust kóde, ani v tom, ako appka reálne synchronizuje/pushuje dáta, sa nedotklo.

## Testy a build

```
cargo test --lib -> 470 passed, 0 failed (bez zmeny - žiadny Rust kód sa nemenil)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.26 build" v hlavičke)
```

Naviac som si to aj vizuálne overil cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) — screenshoty oboch kariet, svetlý aj tmavý režim, dve rôzne šírky okna, plus priama kontrola cez DOM, že `title` na každom tlačidle sedí presne s pôvodným textom. Jeden screenshot (tmavý režim) prikladám, nech vidíš presne, ako to teraz vyzerá, ešte pred tým, ako appku aktualizuješ.

## Zmenené súbory

**Zmenené:**
- `src/pages/Settings.tsx` — `SheetsConnectionCard`: odstavce popisov nahradené jednou krátkou vetou + `title` tooltipmi na tlačidlách, tlačidlá preskupené do 3 riadkov

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.26`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.26 hotové a overené (470/470 testov, čisté `tsc`/`build`, vizuálne overené). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Settings → Integrations. Obe karty (Pulls, Orders & Sales) by mali mať len 1 krátku vetu popisu, potom polia na URL/tab, potom 3 riadky tlačidiel.
2. Podrž myšou nad ľubovoľným tlačidlom (napr. "Sync now" alebo "Push orders") — mala by sa zobraziť bublina s vysvetlením, čo presne to tlačidlo robí.
3. Skontroluj, či ti zoskupenie tlačidiel dáva zmysel — ak by si chcel iné poradie/zoskupenie, napíš mi.
