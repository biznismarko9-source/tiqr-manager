# TIQR Manager 2.0.47 — Dashboard redesign, kolo 1: KPI trendy, Sales by platform, upozornenie na nepredaný inventár

## Čo si mi napísal

Najprv si chcel, aby som nezasahoval do dashboardu a namiesto toho prešiel celý internet — podobné appky na
predaj lístkov, moderné SaaS dashboardy, čo sa dá reálne pridať — a priniesol ti možnosti, nie rovno hotové
zmeny. To som spravil ako samostatný interaktívny dokument (tri vizuálne smery, katalóg widgetov, zdroje).

Potom si vybral konkrétne: **DIR-001** (smer "Ledger Clean" — pokojný, finančný, farba len na stav), **tržby
podľa platformy**, **upozornenie na nepredaný inventár blížiaceho sa eventu**, a **Možnosť A** (rozšíriť
existujúci vlastný SVG graf, nepridávať žiadnu knižnicu na grafy). Toto je prvé kolo implementácie presne
týchto štyroch vecí — funkčnosť appky som nikde nemenil, len pridával.

## Čo je nové

**Karty s číslami na Overview majú teraz trend oproti minulému obdobiu.** Pod veľkým číslom (Revenue, Purchase
cost, Profit, Margin, ROI, Tickets sold) je teraz malý riadok typu "↑ 12.4 % vs. previous period" — porovnáva
aktuálne zvolené obdobie s rovnako dlhým obdobím tesne pred ním (napr. "Posledných 7 dní" sa porovná s
predchádzajúcimi 7 dňami). Farba (zelená/červená) sa ukazuje len tam, kde "hore" jednoznačne znamená
"lepšie" — pri Purchase cost je šípka vždy neutrálne sivá, lebo vyššie náklady nie sú samé osebe zlé. Pri
"All time" alebo Custom bez zadaného dátumu trend jednoducho nie je (niet s čím porovnávať).

**Revenue, Profit a Tickets sold majú pod číslom aj malý graf (sparkline).** Použije presne tie isté dáta, čo
už appka počíta pre veľký graf nižšie — žiadny nový výpočet na pozadí, len to isté číslo zobrazené aj ako
tvar. Kreslené rovnakým spôsobom ako existujúci graf (čistý SVG, žiadna knižnica) — presne to, čo si vybral
ako Možnosť A.

**Nový widget "Sales by platform" na Overview.** Objednávky aj predaje appka už dávno ukladá s platformou —
len sa nikdy nezoskupovali. Teraz je pod hlavným grafom zoznam platforiem zoradený podľa tržieb (najviac
zarábajúca hore), s malým pruhom nech je rozdiel vidieť aj vizuálne, nielen v čísle. Presne princíp, čo
Eventbrite volá "Sales by Source". Rešpektuje ten istý filter obdobia ako karty a graf nad ním.

**Upcoming events (Activity tab) teraz upozorňuje, keď sa event blíži.** Použil som presne ten istý mechanizmus,
čo appka už má pri Pulls (upozornenie 3 dni vopred, každý deň potom, oranžová → červená), len naviazaný na
nepredané lístky blížiaceho sa eventu namiesto termínu odovzdania pull-u. Appka to už raz vyriešila, tak som to
len znova použil na to isté miesto v kóde (skopírované, nie zdieľané — vysvetlené nižšie).

Nikde inde som nič nemenil — Layout, sidebar, ostatné stránky (Events/Orders/Sales/Tickets/Pulls), Financials aj
zvyšok Activity tabu sú presne také, ako boli. Aj samotná farebnosť je tá istá, čo appka už má (modrá `brand`,
zelená/červená len na stav) — ukázalo sa, že appkina existujúca modrá je prakticky totožná s tým, čo som
navrhol v DIR-001, takže som nemusel meniť ani pridávať žiadnu farbu.

## Jedna technická poznámka k tomu, ako je to postavené

Warning na Upcoming events (`daysUntil`/`warningLabel`) je v kóde Dashboardu skopírovaný, nie zdieľaný s
Pulls.tsx, kde presne tento mechanizmus vznikol. Urobil som to zámerne — dohodli sme sa, že toto kolo sa
netýka žiadnej inej stránky ako Dashboard, a Pulls.tsx je hotový, odskúšaný kód, ktorý som nechcel otvárať len
kvôli tomu, aby som odtiaľ vytiahol 15 riadkov do spoločného miesta. Je to pár riadkov duplicity navyše, ale
nulové riziko pre Pulls. Ak chceš, nabudúce sa to dá zjednotiť do jedného spoločného miesta bez zmeny správania.

## Ako som to overoval

Rovnaké obmedzenie ako pri každej appke doteraz: moje prostredie sa nevie pripojiť na `crates.io` ani stiahnuť
balíčky z `npm` (len metadáta prejdú, samotné súbory appky vracajú 403), takže `cargo test`/`npm run build`
som znova nemohol spustiť naostro. Namiesto toho:

- Každý upravený `.rs` súbor prešiel samostatnou syntaktickou kontrolou (`rustc` nad jedným súborom) — skutočné
  chyby v syntaxi by sa takto ukázali, chýbajúce závislosti (očakávané mimo skutočného Cargo projektu) som
  ručne odlíšil od nich.
- Každý upravený `.ts`/`.tsx` súbor prešiel čistou syntaktickou kontrolou cez skutočný TypeScript kompilátor
  (bez sieťovej závislosti na tomto repozitári konkrétne) — 0 chýb vo všetkých piatich súboroch.
- Ručne som prepočítal SQL parametre (`?1`, `?2`, ...) v každom novom dotaze, nech sedia na správne miesto vo
  `Vec` s hodnotami — presne tá trieda chyby, čo sa v tomto kóde dá ľahko pomýliť a kompilátor by ju bežne
  chytil.
- Napísal som 8 nových testov (predtým 570, teraz 578 — počítané priamo cez appkin vlastný zdrojový kód, nie
  prevzaté z minulého reportu) pokrývajúcich: predchádzajúce obdobie sa počíta správne pri prechode cez
  mesiac/rok, "All time" nemá porovnanie, zoskupenie podľa platformy vrátane predaja bez platformy, a že
  refundované predaje sa do "Sales by platform" nepočítajú (rovnako ako všade inde v appke).
- Naviac som dal celú zmenu skontrolovať dvom nezávislým "druhým párom očí" (jeden len na backend Rust kód,
  druhý len na frontend TypeScript/React) bez toho, aby videli moje vlastné zdôvodnenie — presne preto, aby
  našli veci, čo by som si sám neuvedomil. Backend vyšiel úplne čisto. Frontend kontrola našla jednu skutočnú
  chybu, ktorú som hneď opravil: keď sa Profit v predchádzajúcom období presne rovnal nule a v aktuálnom je
  strata, trend by nesprávne ukázal zelenú šípku hore namiesto červenej dole (keďže od nuly sa percento nedá
  spočítať, kód si "hore" vybral automaticky bez ohľadu na znamienko). Opravené a znova overené.

Mimo tejto zmeny som pri kontrole natrafil aj na jednu drobnosť nesúvisiacu s dashboardom: v
`src/lib/format.ts`, vo funkcii `formatSeatsSummary` (existuje od verzie 2.0.38), je namiesto medzery medzi
sekciou a radom uložený neviditeľný nulový bajt. Overil som, že je to už v pôvodnom 2.0.46 zdrojovom kóde, čo
si mi poslal — nespôsobila to táto zmena. Appke to nič nekazí (funguje to ako oddeľovač rovnako spoľahlivo ako
medzera, len je to nezvyčajné), ale kvôli tomu napríklad `git diff` alebo GitHub berie ten súbor ako binárny.
Nechal som to bez zásahu, lebo to je mimo rozsahu tohto kola — ale daj vedieť, ak to mám v budúcom kole opraviť.

## Čo teraz urobiť

1. Nainštaluj 2.0.47.
2. Na Dashboard → Overview skontroluj, či trend pod kartami a malé grafíky na Revenue/Profit/Tickets sold
   sedia s tým, čo poznáš z reálnych dát (skús prepnúť obdobia — Today, 1 Wk, 1 Mo...).
3. Pozri si nový "Sales by platform" pod hlavným grafom — ak máš predaje cez viac platforiem, over, či poradie
   a čísla sedia.
4. Na Dashboard → Activity, sekcia "Upcoming events", skontroluj, či sa oranžové/červené upozornenie objavuje
   pri eventoch do 3 dní.
5. Daj vedieť, ako to vyzerá — toto je prvé kolo, ďalšie veci (grafy, Events/Orders v novom štýle) pridávame
   postupne, presne ako sme sa dohodli.

## Testy a build

```
cargo test --lib  -> nemohol som spustiť naostro (žiadny prístup na crates.io v tomto prostredí)
                      - 578 testov v zdrojovom kóde (o 8 viac ako predtým), vrátane 8 nových pre toto kolo
npx tsc -b        -> nemohol som spustiť naostro (žiadny prístup na npm balíčky tejto appky)
                      - všetkých 5 upravených .ts/.tsx súborov prešlo čistou syntaktickou kontrolou (0 chýb)
```

Skutočný `cargo test`/`tsc -b`/`npm run build` prebehne u teba počas `1-CLICK-UPDATE.bat` (GitHub Actions) —
to zostáva jediné miesto, kde sa toto overí naozaj.

## Zmenené súbory

**Backend:** `src-tauri/src/models.rs` (nový `PlatformSales`, dve nové polia na `DashboardData`),
`src-tauri/src/commands/dashboard.rs` (nová logika pre predchádzajúce obdobie a tržby podľa platformy, 8
nových testov).

**Frontend:** `src/lib/types.ts` (nový `PlatformSales`, dve nové polia), `src/lib/format.ts` (nové
`computeTrend`/`computeTrendPoints`), `src/components/icons.tsx` (3 nové ikony), `src/components/ui.tsx`
(`StatCard` rozšírený o voliteľný trend/sparkline — Event Detail, čo `StatCard` tiež používa, zostáva úplne
nezmenené), `src/pages/Dashboard.tsx` (hlavná zmena — nové karty, nový widget, nové upozornenie).

**Verzia (7 miest):** `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
`src-tauri/Cargo.lock`, `release.ps1` (aj text commit správy), `1-CLICK-UPDATE.bat` — všetkých na `2.0.47`.
