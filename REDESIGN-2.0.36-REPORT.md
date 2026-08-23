# TIQR Manager 2.0.36 — hlavičky aj dáta v tabuľkách už nie sú tesné nikde

## Čo je nové

Presne to, čo si nahlásil na oboch screenshotoch (Pulls aj Sales) - "Sale Event Platform Date Tix Revenue Fees Cost Profit Margin ROI Status" nebolo všade celé vidno, aj keď bolo dosť miesta. Prešiel som to poriadne, nielen na Sales, ale úplne všade, kde appka niečo zapisuje do tabuľky - Sales, Sale Detail, Orders, Order Detail, Events, Tickets/Inventory, Pulls (obe podzáložky). Skontroloval som aj Event Detail, tá ale funguje úplne inak (nemá pevné šírky stĺpcov, prispôsobuje sa sama) - tam sa táto chyba stať nemôže, netreba nič opravovať.

Tento report rieši **len** to hlásenie o tesných stĺpcoch. Sťahovaciu obrazovku (namiesto tej z Windowsu), obrazovku pre update v appke a "bid obrazok" vizuál som si nechal na ďalšie kolo - je to väčšia, samostatná vec (nová obrazovka od nuly), a chcel som mať istotu, že tabuľky sú poriadne opravené predtým, než na to nabalím ďalšiu prácu. Zatiaľ som len zistil, že appka MÁ reálne zapojený mechanizmus na aktualizácie (`tauri-plugin-updater`, mieri na tvoj GitHub release), takže je na čom stavať - konkrétny dizajn tej obrazovky príde v ďalšej verzii.

## Prečo sa to stalo - a prečo to nebolo vidno hneď

Keď som sa na to pozrel prvýkrát, skontroloval som len to, či sa zmestí samotný text hlavičky ("Cost", "Margin", ...) do stĺpca. Podľa toho to vyzeralo, že drvivá väčšina stĺpcov je v poriadku - anglické slovo "Cost" je krátke, zmestí sa ľahko.

Až keď som skontroloval, čo sa reálne PÍŠE do tých stĺpcov (nie hlavičku, ale skutočné sumy/percentá/dátumy), ukázalo sa, že skoro každý peniazový/percentový/dátumový stĺpec bol v skutočnosti príliš úzky - len to nebolo vidno na anglickom texte hlavičky. Dôvod: tvoj Windows je s vysokou pravdepodobnosťou nastavený na slovenčinu, a slovenský formát peňazí ("99 999,99 $") je citeľne širší než anglický ("$99,999.99") - medzera namiesto čiarky, desatinná čiarka namiesto bodky, symbol meny na konci namiesto na začiatku. To isté pri dátumoch - skrátený názov mesiaca je v slovenčine dlhší ("sept." namiesto "Sep"). Overil som to reálnym meraním v troch jazykových nastaveniach (anglicky, slovensky, nemecky), nie odhadom.

Takže dôvod, prečo si videl orezané/zalomené hlavičky, bol v skutočnosti dvojaký:
1. Niektoré hlavičky (Platform, Margin/ROI na Sales) boli tesné už len na anglický text.
2. Oveľa viac stĺpcov (Cost, Revenue, Fees, Profit, Date, a ďalšie) malo v poriadku hlavičku, ale bolo tesných na reálne čísla/dátumy, ktoré appka do nich píše - to sa prejaví práve u teba, nie na anglicky nastavenom počítači, čo je presne dôvod, prečo to pri prvej kontrole nebolo vidno.

Mimochodom - Sales/Sale Detail/Orders/Order Detail (tie, čo minulá verzia prerobila na percentá namiesto pevných pixelov) mali ešte jeden samostatný nedostatok: percentá samé osebe zaručujú len POMER medzi stĺpcami, nie minimálnu šírku. Na užšom okne (menšom než pôvodných 1400px, na ktoré boli počítané) sa mohli všetky stĺpce scvrknúť ešte viac a byť tesné odznova. Pridal som im teda aj minimálnu šírku tabuľky - presne to, čo Pulls tabuľka mala už od jesene minulého roka.

## Čo presne sa zmenilo

- **Sales, Sale Detail, Orders, Order Detail** - stĺpce s peniazmi/percentami/dátumami/číslami rozšírené (Tix, Revenue, Fees, Cost, Profit, Margin/ROI, Status na Sales; podobne na ostatných troch), plus pridaná minimálna šírka tabuľky, aby sa už nemohli scvrknúť pod bezpečnú hranicu. Stĺpec Event (resp. Seat na Sale/Order Detail) zostáva aj po zmene jednoznačne najširší v riadku.
- **Events, Tickets/Inventory, Pulls (obe podzáložky)** - rovnaký princíp, len tieto tabuľky majú pevné pixelové šírky namiesto percent, tak sa upravili priamo čísla v kóde. Pulls obe podzáložky mali už predtým nastavenú minimálnu šírku celej tabuľky (kvôli staršej oprave) - tú som primerane zväčšil, nech stĺpec Event neprde o priestor, ktorý mal doteraz.
- Stĺpce, ktoré len skracujú dlhý text bodkami (Event, Platform, Seat, Notes, ...) som nechal tak, ako sú - tie majú vlastnú poistku (bodky + názov po podržaní myšou), to nie je tá chyba, čo si hlásil.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - táto oprava sa Rust kódu netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.36 build" v hlavičke)
```

Kontrola nebola len na oko - písal som si skript (Playwright), ktorý do bežiacej appky vloží skutočné hlavičky AJ skutočne naformátované dáta (peniaze/percentá/dátumy, v troch jazykových nastaveniach) a odmeria, či sa zmestia do stĺpca. Skontrolované na viacerých šírkach okna (1920/1400/1100px a navyše presne pri novej minimálnej šírke každej tabuľky) - všade bez výnimky sa teraz zmestí aj hlavička, aj najhorší reálny prípad dát.

Mimochodom, keď už appka bežala, overil som aj predchádzajúce tri verzie (2.0.33, 2.0.34, 2.0.35), ktoré si mi poslal - predtým ich nikto reálne neskompiloval ani neotestoval (vznikli v inom prostredí, ktoré na to nemalo prístup). Dobrá správa: všetky tri prešli čisto (491/491 testov, `tsc`/`build` bez chýb) - nič z toho nebolo pokazené, len tento konkrétny problém so stĺpcami sa dovtedy neopravil poriadne (dva pokusy predtým, v 2.0.33 a 2.0.34, opravili vždy len jeden konkrétny stĺpec, nie celý vzor).

Zip zabalený z presného zoznamu 196 súborov (nič sa neuberalo, pribudol len tento report), vybalený do prázdneho priečinka, tam znova `npm ci` + `tsc -b` + `npm run build` (všetko čisté) a porovnaný bajt po bajte s mojím pracovným priečinkom - sedí presne.

## Zmenené súbory

**Frontend (7 súborov):**
- `src/pages/Sales.tsx`, `src/pages/SaleDetail.tsx`, `src/pages/Orders.tsx`, `src/pages/OrderDetail.tsx` - rozšírené stĺpce + pridaná minimálna šírka tabuľky
- `src/pages/Events.tsx`, `src/pages/Tickets.tsx`, `src/pages/Pulls.tsx` (obe podzáložky) - rozšírené stĺpce, u Pulls aj zväčšená minimálna šírka tabuľky

**Verzia (8 miest):** `package.json`, `package-lock.json` (2×), `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version`), `1-CLICK-UPDATE.bat` - všetkých na `2.0.36`.

## STOP

2.0.36 hotové a overené (491/491 testov, čisté `tsc`/`build`, kontrola cez skutočné dáta v troch jazykoch, nielen anglický text hlavičky). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Sales aj Pulls (presne tie dve, čo boli na tvojich screenshotoch) - hlavičky by teraz mali byť celé, bez zalamovania.
2. Skontroluj aj Orders, Order Detail, Events, Tickets/Inventory - rovnaký princíp opravy platí všade.
3. Ak máš nejaké eventy/objednávky s vyššími sumami alebo percentami (napr. veľký ROI), skontroluj aj tie riadky - presne na také prípady bola oprava mierená.
4. Sťahovacia obrazovka / update obrazovka / "bid obrazok" - toto v 2.0.36 ešte nie je, príde v ďalšej verzii (vysvetlené vyššie prečo).
