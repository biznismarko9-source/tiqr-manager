# TIQR Manager 2.0.32 — tabuľky a súhrnný riadok už nie sú zbytočne roztiahnuté

## Čo je nové

Presne to, na čo si narazil na Sales - keď je appka maximalizovaná (alebo na veľkom monitore), tabuľka aj riadok so súhrnom ("Results / Tickets / Revenue / ...") a triedením sa síce roztiahli na celú šírku (to bola zámerná zmena z 2.0.31), ale keďže dáta v tabuľke na to nestačia, vznikol veľký prázdny pás - hlavne v stĺpci Event, a medzi súhrnom a triedením "Newest first".

Opravené na všetkých stránkach s tabuľkou naraz - Sales, Sale Detail, Orders, Order Detail, Events, Event Detail, Tickets/Inventory, Pulls (obe podzáložky) - je to jeden opakujúci sa vzor, tak isto ako 2.0.31 bola jedna spoločná zmena pre všetky stránky.

## Ako presne to funguje pod kapotou

2.0.31 odstránila strop zo *stránky* - to bolo správne, presne si to chcel. Problém je inde: každá tabuľka v appke má pevne dané šírky pre väčšinu stĺpcov a JEDEN stĺpec (najčastejšie Event), ktorý nasáva všetku zvyšnú šírku. Kým bola stránka capnutá na 1400px, ten jeden stĺpec dostal rozumný zvyšok. Keď stránka od 2.0.31 vyplní aj 1920px+ okno, presne ten istý mechanizmus mu dá stovky pixelov navyše, ktoré mu na nič nie sú - "Spain vs England" nepotrebuje 950px široký stĺpec.

Riešenie nie je vrátiť strop na celú stránku (to by ťa vrátilo presne k problému z 2.0.31), ale dať rozumný strop LEN tabuľke (a súhrnnému riadku nad ňou, nech "Newest first" nelieta zbytočne ďaleko od tabuľky, ktorú ovláda). Nadpis stránky, tlačidlá (Delete / New Sale) a riadok filtrov zostávajú roztiahnuté na celú šírku presne ako v 2.0.31 - tam žiadny podobný problém nie je, lebo tam nič nič "nenasáva" všetku voľnú šírku. Použil som rovnakú šírku 1400px, akú mala predtým celá stránka - tá bola overená ako vyzerajúca dobre cez desiatky predchádzajúcich verzií, len je teraz naviazaná na tabuľku samotnú, nie na celú stránku.

Výsledok: na tvojom veľkom okne teraz vidíš plnú šírku pre hlavičku aj filtre (žiadna medzera vôkol celej stránky, presne ako chceš), a tabuľka pod tým má rozumnú, čitateľnú šírku namiesto toho, aby vyzerala prázdna. Na bežnom/menšom okne (do 1400px) sa nezmenilo vôbec nič - tabuľka aj predtým vypĺňala presne toľko miesta, koľko mala.

Event Detail má navyše dve tabuľky (Orders, Tickets), ktoré nepoužívajú presne ten istý mechanizmus (iný typ tabuľky), ale majú rovnaký základný problém (natiahnu sa na celú šírku) - dostali ten istý strop pre konzistentnosť.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - tato oprava sa Rust kódu vôbec netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.32 build" v hlavičke)
```

Pred úpravou aj po nej overené vizuálne a číselne cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) - presná kópia Sales tabuľky aj s tvojimi dátami zo screenshotu ("Spain vs England", Viagogo, 8 lístkov...), na 1280px/1920px/2200px oknách. Pred opravou: stĺpec Event 950px široký pri 1920px okne (obsah potrebuje ~180px). Po oprave: tabuľka drží presne 1400px, normálne okno (1280px) úplne bez zmeny. Rovnaká kontrola zopakovaná aj na Pulls tabuľke (iný počet stĺpcov, iný pôvodný `min-width`) pre istotu, že sa to správa rovnako všade.

Zip bol znova zabalený z presného zoznamu 191 súborov (nič sa nepridalo, nič sa neuberalo - len úpravy existujúcich súborov), vybalený do prázdneho priečinka, tam znova `npm ci` + `tsc -b` + `npm run build` (všetko čisté), a porovnaný bajt po bajte s mojím pracovným priečinkom - sedí presne.

## Zmenené súbory

**Frontend (11 súborov, všade rovnaký `max-w-[1400px]` na obale tabuľky):**
- `src/pages/Sales.tsx` - tabuľka + súhrnný/triediaci riadok
- `src/pages/SaleDetail.tsx`, `src/pages/Orders.tsx`, `src/pages/OrderDetail.tsx`, `src/pages/Events.tsx`, `src/pages/EventDetail.tsx` (obe tabuľky), `src/pages/Tickets.tsx` (aj tabuľka, aj riadok filtrov kvôli súhrnu "N orders · N tickets..."), `src/pages/Pulls.tsx` (obe podzáložky) - len obal tabuľky

**Verzia (8 miest):** `package.json`, `package-lock.json` (2×), `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version`), `1-CLICK-UPDATE.bat` - všetkých na `2.0.32`.

## STOP

2.0.32 hotové a overené (491/491 testov, čisté `tsc`/`build`, overené aj číselne aj vizuálne pred/po). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Na Sales (maximalizované okno) - tabuľka aj súhrnný riadok by mali vyzerať kompaktnejšie, bez veľkej prázdnej medzery v stĺpci Event.
2. Skontroluj aj Orders, Events, Tickets/Inventory, Pulls - rovnaká zmena platí všade.
3. Filtre a hlavička hore by mali stále vypĺňať celú šírku okna (to sa NEMENILO) - len samotná tabuľka pod nimi je teraz užšia.
