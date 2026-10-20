# TIQR Manager 2.0.23 — Pull info doplnené neskôr sa teraz tiež prepojí do Received Pulls

## ⚠️ Over si, či som to pochopil správne

Písal si: *"pri orders viem vyplnit aj tieto 3 info o pulle pull, who pulled, how much pull, ked sa to vyplni tak automaticky sa to da do received pulls"*.

Tieto 3 stĺpce (`pull`, `who pulled`, `how much pull`) **existujú len v pripojenom Google Sheete** (hárok Orders & Sales) — v appke samotnej (obrazovka Orders / detail objednávky) som nenašiel žiadne políčko na priame vypĺňanie pull informácií, appka ich vôbec nezobrazuje pri objednávke. Predpokladal som teda, že hovoríš o vypĺňaní v hárku (presne v duchu všetkého, čo sme tento týždeň robili — Platform/Transfer dropdowny, farby, atď.).

**Ak si namiesto toho myslel niečo iné — že by aj samotná appka (obrazovka Orders) mala mať políčka na pull/who pulled/how much pull priamo pri objednávke, nezávisle od hárku — napíš mi to, je to iná (väčšia) úloha, ktorú som nerobil.**

## 1. Čo som zistil — toto vlastne čiastočne už fungovalo

Automatické prepojenie do "Received Pulls" pri vyplnení `pull=yes` + `who pulled` v hárku **existuje už od verzie 2.0.17** — nie je to nová vec. Keď v tom istom riadku vyplníš predajné stĺpce (Payout Per Ticket a pod.) AJ pull stĺpce naraz a potom spustíš "Sales sync", appka:
- vytvorí predaj, presne ako doteraz,
- **a rovno aj záznam v Received Pulls**, prepojený na tú objednávku.

Toto som si overil, funguje to a nemenil som to.

## 2. Čo bola tá skutočná diera — a čo som opravil

Objavil som ale medzeru presne v tom scenári, ktorý si popísal: **ak riadok najprv zosynchronizuješ ako predaný (bez pull info), a až POTOM sa vrátiš do hárku a dopíšeš `pull`/`who pulled`/`how much pull` do toho istého (už predaného) riadku a spustíš Sales sync znova — doteraz sa nestalo vôbec nič.** Appka takýto riadok označila ako "už hotový, nič nové" a pull stĺpce si už vôbec ani nepozrela.

Teraz to appka opravuje: **Sales sync teraz kontroluje pull/who pulled/how much pull pri KAŽDOM spustení, nielen v tom, čo predaj prvýkrát vytvorí.** Takže:

1. Vyplníš predajné stĺpce → Sales sync → predaj sa vytvorí (ako doteraz).
2. Kedykoľvek neskôr dopíšeš `pull` = Yes, `who pulled` a `how much pull` do toho istého riadku → Sales sync znova → **teraz sa automaticky vytvorí prepojený záznam v Received Pulls**, presne ako keby si to vyplnil hneď napoprvé.

Toto sa v číslach po synchronizácii ukáže ako "1 updated" (nie "1 created" — samotný predaj sa predsa nevytváral nanovo).

## 3. Čo som nemenil

- Ak `pull` nie je presne "yes", alebo `who pulled` je prázdne → stále sa nič nevytvorí (rovnako ako doteraz).
- Ak objednávka nemá vôbec nič predané (napr. všetky lístky zrušené) → pull sa nikdy nepokúsi prepojiť, aj keby si tam pull=yes napísal — nemá k čomu (žiadne "how many tickets" by to malo vypĺňať).
- Raz prepojený záznam sa nikdy neprepisuje ani neduplikuje, nech riadok zosynchronizuješ koľkokrát chceš — to isté poistenie ako predtým, len teraz sa skúša pri každom behu, nie len pri prvom.
- Množstvo (quantity) v novom zázname teraz správne berie skutočný počet predaných lístkov tej objednávky (nie 0, čo by sa stalo, keby appka počítala len "práve teraz predané").
- Nič iné v hárku, appke ani v UI som sa nedotkol — žiadne nové políčka, žiadne zmeny v Orders/Sales obrazovke.

## 4. Testy a build

```
cargo test --lib -> 461 passed, 0 failed (bolo 457 - pribudli 4 nové testy)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.23 build" v hlavičke)
```

Nové testy okrem iného overujú presne tvoj scenár (predaj sync, potom neskôr dopísaný pull, potom znova sync → prepojí sa), že sa to nezduplikuje pri treťom synci, že množstvo je správne pri viacerých lístkoch, a že sa nič nestane, keď objednávka nič nepredala.

## 5. Zmenené súbory

**Zmenené:**
- `src-tauri/src/commands/orders_sheet_sync.rs` — `apply_sales_rows` teraz kontroluje pull/who pulled/how much pull aj na riadku, ktorý je už celý predaný; `maybe_link_pull_received` vracia, či naozaj niečo prepojila (aby sa to vedelo započítať ako "updated")

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.23`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.23 hotové a overené (461/461 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj presne tvoj scenár:

1. V hárku Orders & Sales vyplň bežný predaj (Payout Per Ticket, Status, atď.) na jednom riadku, ale **pull stĺpce nechaj prázdne**. Spusti "Sales sync" → predaj sa vytvorí, v appke v Pulls → Received Pulls nič nové.
2. Vráť sa do toho istého riadku a dopíš `pull` = Yes, `who pulled` = meno, `how much pull` = suma. Spusti "Sales sync" znova.
3. Skontroluj Pulls → Received Pulls v appke — mal by tam pribudnúť nový záznam, prepojený na tú objednávku, so správnym menom/sumou/počtom lístkov.
4. Spusti Sales sync ešte raz (bez zmeny riadku) — v Received Pulls by nemal pribudnúť žiadny duplicitný záznam.

A hlavne — potvrď mi bod ⚠️ na začiatku, či si myslel presne toto (hárok), alebo či chceš aj políčka priamo v appke pri objednávke.
