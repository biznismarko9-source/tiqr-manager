# TIQR Manager 2.0.11 — Orders & Tickets: kompletná hlavička pri "Create a new sheet for me"

Krátka oprava presne na to, čo si napísal: tlačidlo "Create a new sheet for me" na karte Orders & Tickets písalo do nového hárku len pôvodných 13 stĺpcov (tie pre Order sync), nie aj tých 12 pre Sales sync. Teraz píše všetkých 25, presne v poradí, ako si ich napísal:

`Event Name`, `Date (DD/MM/YYYY)`, `platform`, `Section`, `Row`, `Seats`, `Order ID`, `Total Purchase Price`, `Number of Tickets`, `Price Per Ticket`, `currency`, `Email (used)`, `Ticket Type`, `Site Listed`, `Payout Per Ticket`, `Revenue`, `Profit`, `Status`, `Delivery status`, `Payout status`, `date of purchase`, `paid by`, `pull`, `who pulled`, `how much pull`

Prvých 13 je nezmenených (Order sync), zvyšných 12 pribudlo (Sales sync) - takže nový hárok je hneď pripravený na obe tlačidlá naraz, bez ručného dopĺňania.

## 1. Potvrdené - "date of purchase" je správny názov stĺpca s dátumom predaja

V 2.0.10 reporte (bod 3) som písal, že som si nebol istý presným názvom stĺpca s dátumom predaja, a radšej som nehádal. Tvoja správa ho teraz potvrdila presne ako "date of purchase" - appka ho už aj predtým skúšala ako jeden z uznávaných názvov, takže tu nič meniť netreba, len je to teraz overené a nie len môj odhad.

## 2. Revenue a Profit sú v hlavičke, appka ich stále nečíta

Pripomienka z 2.0.10 (tvoja vlastná voľba, nezmenené): stĺpce `Revenue` a `Profit` sú teraz v hlavičke nového hárku, aby si ich mal na svoj vlastný prehľad, ale appka ich stále vôbec nečíta - profit/revenue/maržu si appka vždy sama dopočíta z ceny predaja a nákladov na lístok, aby sa nemohlo stať, že číslo v tabuľke a číslo v appke časom prestanú sedieť.

## 3. Drobná technická poistka - appka teraz číta o kus širší rozsah stĺpcov

Toto si nepýtal, ale súvisí to priamo s touto zmenou, tak som to rovno opravil aj s tým: appka doteraz čítala z hárku len stĺpce A až Z (26 stĺpcov). Kým bolo v hlavičke 13 stĺpcov + 1 skrytý "TIQR ID" (appka si ho pridáva sama), bola tam veľká rezerva. Teraz je to 25 stĺpcov + "TIQR ID" = presne 26 - čiže presne na hranici, s nulovou rezervou pre akýkoľvek ďalší stĺpec, čo by si si niekedy pridal. Rozšíril som to na A až AZ (52 stĺpcov), aby bola opäť poriadna rezerva a nestalo sa, že appka niekedy v budúcnosti niečo z hárku ticho "nevidí".

## 4. Čo urob teraz

Záleží, či už ten automaticky vytvorený hárok reálne používaš:

1. **Ak v ňom ešte nemáš dôležité dáta** (alebo ti neprekáža začať nanovo): choď do Settings -> Integrations -> karta "Orders & Tickets" -> **"Create a new sheet for me"** znova. Appka vytvorí úplne nový hárok so všetkými 25 stĺpcami a hneď naň appku prepojí namiesto toho starého. Starý hárok v Google Sheets nezmizne, appka na neho len prestane byť napojená.
2. **Ak už v tom hárku máš reálne dáta** a nechceš prepínať: pridaj tam ručne tých 12 chýbajúcich stĺpcov za "Ticket Type", presne v tomto poradí a s presne týmito názvami (appka nerozlišuje veľké/malé písmená, ale text musí sedieť): `Site Listed`, `Payout Per Ticket`, `Revenue`, `Profit`, `Status`, `Delivery status`, `Payout status`, `date of purchase`, `paid by`, `pull`, `who pulled`, `how much pull`.

Ak namiesto toho používaš svoj vlastný, inak založený hárok (nie ten cez tlačidlo vytvorený), táto zmena sa ho netýka - len over, že v ňom sedia tieto názvy (alebo niektorý z uznávaných alternatívnych názvov z 2.0.10 reportu, bod 2/3).

## 5. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 332 passed, 0 failed (330 + 3 zmenené/nové)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.11 build" v hlavičke)
```

Jeden starý test som musel prerobiť - kontroloval, že hlavička nového hárku je *presne* rovnaká ako 13-stĺpcová testovacia sada pre Order sync, čo už po tejto zmene neplatí doslovne. Nahradil som ho presnejšou kontrolou: že prvých 13 stĺpcov sedí do písmena tak ako predtým (Order sync nič nestratil), plus nový test, že celá 25-stĺpcová hlavička aj so svojím "TIQR ID" spĺňa aj vlastné požiadavky Sales sync-u, plus nový test priamo na "date of purchase" ako reálny, potvrdený názov stĺpca.

## 6. Zmenené súbory

**Zmenené:** `src-tauri/src/commands/orders_sheet_sync.rs` (`ORDERS_SHEET_HEADERS` rozšírený z 13 na 25 stĺpcov, rozsah čítania z hárku rozšírený A1:Z -> A1:AZ, 1 test prerobený + 2 nové)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.11`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.11 hotové a overené (332/332 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat`, potom buď klikni "Create a new sheet for me" znova (ak môžeš začať na novom hárku), alebo pridaj tých 12 stĺpcov ručne do toho existujúceho (bod 4 vyššie) - a napíš mi, či hlavička teraz presne sedí s tým, čo potrebuješ.
