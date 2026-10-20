# TIQR Manager 2.0.42 — EUR pri výpočtoch, oprava 2 chýb a automatická oprava cien pri automatizovaných objednávkach

## Čo si mi napísal

Poslal si mi 2 správy so screenshotmi:

1. Pri Summary bloku (Orders & Sales): *"je tu mensi problem, pri tychto vypoctoch musi byt priradena aj
   mena a tu daj do eur, a taktiez oprav tie 2 errory co su"*
2. Kúsok nato, pri výsledku synchronizácie: *"taktiez niekedy sa stane v google sheets lebo tie orders su
   zautomatizovane ze ta cena nesedi uplne do centu, nechcem, aby ukazalo error, ze to musis opravit, ale
   chcem, aby to apka sama opravila a posunula to do dashboradu a taktiez updatla v google sheets, jasne
   nemoze tam napisat hlupost, musi to davat zmysel"*

Toto je oprava/rozšírenie na obe naraz - poďme si to prejsť po jednom.

## 1. Prečo Total Paid / Total Unpaid hádzali #ERROR!

Google Sheets si vzorce parsuje podľa jazykového nastavenia SAMOTNEJ tabuľky (nie appky) - a keď má
tabuľka nastavenú desatinnú čiarku (presne tvoj prípad, vidno to napríklad na "3488,06" v tvojom vlastnom
screenshote), Google Sheets vo vzorcoch namiesto čiarky medzi argumentmi funkcie čaká bodkočiarku, lebo
čiarka je už obsadená ako desatinná čiarka. Vzorec pre Total Paid/Total Unpaid používal `SUMIF` s čiarkami
- na tvojej tabuľke to preto vždy spadlo na #ERROR!.

Namiesto toho, aby appka hádala/zisťovala presné jazykové nastavenie tvojej tabuľky (a mohla sa pomýliť
nabudúce na inej), som vzorec prepísal tak, aby vôbec nepoužíval žiadnu čiarku ani bodkočiarku medzi
argumentmi funkcie - funguje teda rovnako bez ohľadu na to, aký jazyk má tabuľka nastavený.

## 2. EUR pri výpočtoch

V 2.0.40 som tieto bunky nechal ako obyčajné čísla - vtedy som si nebol istý, či appka vie spoľahlivo
trafiť správny formát podľa jazyka tvojej tabuľky. Ukázalo sa, že to bola zbytočná opatrnosť: appke stačí
Google Sheets povedať "toto je mena, 2 desatinné miesta" a zvyšok (medzery, čiarky/bodky) si Sheets
doplní sám, presne podľa toho, ako máš tabuľku nastavenú.

Pridal som teda EUR formát na všetkých 5 vypočítaných bunkách v Summary bloku (Total Cost/Revenue/
Profit/Paid/Unpaid) - a pre poriadok aj na bunku "Total price (€)" v Pulls tabuľke, ktorá mala úplne
rovnaký nedostatok, aj keď si sa na ňu konkrétne nesťažoval.

## 3. Automatická oprava cien pri automatizovaných objednávkach (nová vec)

Keď ti objednávky napĺňa automatizácia, občas sa stane, že Total Purchase Price sa nezhoduje úplne do
centu s Number of Tickets × Price Per Ticket (napr. o 1-2 centy), alebo že Price Per Ticket má viac ako 2
desatinné miesta (napr. 96.6825 namiesto 96.68) - typicky preto, že to vzniklo delením, ktoré nevyšlo na
rovný cent.

Doteraz appka takýto riadok jednoducho preskočila a nahlásila chybu, ktorú si musel riešiť ručne. Odteraz:

- Ak je rozdiel MALÝ (dá sa rozumne vysvetliť bežným zaokrúhľovaním na cent), appka si to opraví sama:
  použije správne číslo, uloží objednávku do appky AJ ju rovno prepíše naspäť do Google Sheets, aby aj
  tam sedelo. Vo výsledku synchronizácie to uvidíš ako zelenú správu "auto-corrected" - nie ako chybu.
- Ak je rozdiel VEĽKÝ (viac, než sa dá vysvetliť zaokrúhľovaním), appka to aj naďalej nahlási ako chybu a
  riadok preskočí, presne ako doteraz. Tvoja požiadavka bola jasná - *"nemoze tam napisat hlupost, musi to
  davat zmysel"* - takže appka si nikdy nič nevymyslí, opraví len to, čo sa dá rozumne vysvetliť ako
  drobná nepresnosť.

Hranicu medzi "toto je len zaokrúhľovanie" a "toto je skutočná chyba" appka počíta matematicky, nie
odhadom: koľko centov môže maximálne "ujsť", keď sa celková suma delí na počet lístkov a výsledok sa
zaokrúhli na cent. Pri objednávke so 4 lístkami je to napríklad maximálne 2 centy - presne toľko, koľko
bol rozdiel aj na tvojom vlastnom screenshote (337.10 vs. vypočítaných 337.12). Ten druhý prípad z tvojho
screenshotu, kde bol rozdiel skoro 3 eurá (401.99 vs. 399.00), appka aj naďalej správne odmietne - to už
nie je bežné zaokrúhľovanie, ale reálny nesúlad, ktorý si zaslúži tvoju pozornosť.

## Čo teraz urobiť

Nič špeciálne ručne netreba - stačí normálne používať Order sync a "Update sheet" na Orders & Sales.
Total Paid/Total Unpaid by teraz mali ukazovať správne čísla namiesto #ERROR!, výpočty by mali mať pri
sebe €, a automatizované objednávky s drobnou nepresnosťou by sa už nemali strácať v chybách. Ak narazíš
na čokoľvek, čo sa nezdá v poriadku (napr. appka niečo opravila a tebe to nesedí), pošli mi prosím
screenshot presne tak ako doteraz - najrýchlejšie mi to pomôže pochopiť, čo sa deje.

## Testy a build

```
cargo test --lib -> 557 passed, 0 failed, 3 ignored (27 nových testov oproti 2.0.41, vrátane testov
                     postavených priamo na tvojich vlastných číslach zo screenshotov)
cargo check --lib -> 0 chýb
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

## Zmenené súbory

**Backend (5 súborov):** `src-tauri/src/money.rs`, `src-tauri/src/google_sheets.rs`, `src-tauri/src/
commands/orders_sheet_sync.rs`, `src-tauri/src/commands/pulls_sheet_sync.rs`, `src-tauri/src/models.rs`.

**Frontend (2 súbory):** `src/lib/types.ts`, `src/pages/Settings.tsx` (nová zelená správa
"auto-corrected" vo výsledku synchronizácie).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.42`.

## STOP

1. Skús Order sync alebo "Update sheet" na Orders & Sales - Total Paid/Total Unpaid by mali ukazovať
   čísla namiesto #ERROR!, a výpočty v Summary bloku (aj Total price v Pulls) by mali mať pri sebe €.
2. Ak máš po ruke nejakú tú automatizovanú objednávku, ktorá sa predtým sťažovala na nepresnú cenu, skús
   ju znova zosynchronizovať - mala by sa teraz buď sama opraviť (a vidno to zeleno v zozname výsledkov),
   alebo, ak je rozdiel naozaj veľký, ostať nahlásená ako chyba presne ako doteraz.
3. Čokoľvek nezvyčajné - pošli mi prosím screenshot, nech to viem rýchlo dohľadať a opraviť.
