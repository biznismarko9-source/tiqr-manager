# TIQR Manager 2.0.22 — Farby pri vybraných možnostiach

## ⚠️ Najprv over si toto — jedna vec som si musel domyslieť

Pri "pri pulls to je yes zelenou, pulls modrou" mi to nedávalo zmysel doslovne (slovo "pulls" tam je dvakrát) — pochopil som to ako preklep a že si myslel **"yes zelenou, nie modrou"** (Transfer: Áno = zelená, Nie = modrá). Ak som to pochopil zle, napíš mi presne, čo si mal na mysli, je to jednoriadková zmena.

## 1. Čo je teraz vyfarbené

**Hárok Pulls — Transfer:**
- Yes → zelená
- No → modrá

**Hárok Orders & Sales:**
- **Status:** Listed → oranžová, Unlisted → hnedá, Sold → zelená
- **Delivery status:** Delivered → zelená, Not delivered → oranžová
- **Payout status:** Pending → oranžová, Paid → zelená

Presne len tieto stĺpce — Ticket Type, Site Listed, pull a Platform (tie majú svoj dropdown z minulých verzií) som farbami nedotkol, keďže si ich nespomenul.

Keďže si zadal len názvy farieb, nie presné odtiene, vybral som rozumné svetlejšie odtiene, nech text v bunke zostane dobre čitateľný. Ak by si chcel inú presnú farbu (tmavšiu/svetlejšiu, alebo úplne inú), stačí napísať a zmením to.

## 2. Ako presne to funguje

Toto nie je farba priamo v dropdown-menu (Google Sheets má aj takú novšiu funkciu — farebné "chipy" v samotnom zozname na výber — ale tá sa cez API, ktoré appka používa, zatiaľ vôbec nedá nastaviť, overil som si to priamo v aktuálnej Google dokumentácii). Appka namiesto toho farbí **pozadie bunky** podľa toho, aká presná hodnota je v nej napísaná — čo je bežný a spoľahlivý spôsob, ako to isté vidieť vo výsledku: keď bunka obsahuje presne "Sold", appka jej dá zelené pozadie, atď.

## 3. Bezpečné pri opakovanom spúšťaní

Appka si pri každom Sync now/Push to sheet/Update sheet sama skontroluje, čo už má vo farbách nastavené, a **nahradí len svoje vlastné predošlé pravidlá** na tých istých stĺpcoch — nič sa nehromadí donekonečna a appka sa nikdy nedotkne farebného pravidla, čo by si si prípadne pridal sám niekde inde v hárku.

## 4. Testy a build

```
cargo test --lib -> 457 passed, 0 failed (bolo 438 - pribudlo 19 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.22 build" v hlavičke)
```

## 5. Zmenené súbory

**Zmenené:**
- `src-tauri/src/google_sheets.rs` — nová `get_sheet_structure_metadata` (nahrádza staršiu `get_sheet_numeric_id` — robí to isté a naviac), `conditional_format_indices_to_replace`, `add_conditional_format_color_request`, `delete_conditional_format_rule_request`
- `src-tauri/src/commands/orders_sheet_sync.rs` — nová `plan_sheet_color_updates` (Status/Delivery status/Payout status), zapojená do všetkých 4 príkazov aj do "Update sheet"
- `src-tauri/src/commands/pulls_sheet_sync.rs` — nová `plan_pulls_sheet_color_updates` (Transfer), zapojená do Sync now/Push to sheet/Update sheet

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.22`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.22 hotové a overené (457/457 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V hárku Pulls, stĺpec Transfer — riadky s "Yes" by mali mať zelené pozadie, "No" modré.
2. V hárku Orders & Sales, stĺpce Status/Delivery status/Payout status — farby presne podľa zoznamu v bode 1 vyššie.
3. Skús to spustiť dvakrát/trikrát po sebe (napr. Sync now, potom hneď znova) — farby by mali zostať rovnaké, žiadne zdvojené/naskladané pravidlá (dá sa to overiť v Format → Conditional formatting v Google Sheets — počet pravidiel na daný stĺpec by sa nemal zvyšovať pri opakovanom spustení).

A hlavne — potvrď mi bod ⚠️ na začiatku, či som "pulls modrou" pochopil správne ako "nie modrou".
