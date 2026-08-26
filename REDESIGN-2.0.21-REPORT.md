# TIQR Manager 2.0.21 — Hárok Pulls: dropdown pre Platform a Transfer

Presne to, čo si chcel pre hárok **Pulls**: `pull`, `Event name`, `event date`, `Ks`, `More info`, `Section`, `Row`, `Seats`, `Price` zostávajú úplne bez zmeny — tie som sa vôbec nedotkol. **Platform** je teraz skutočný dropdown na výber, prepojený s appkou presne tak, ako si opísal. **Transfer** je teraz dropdown s dvoma možnosťami: Áno/Nie.

## Žiadne riziko prepísania — na rozdiel od minule

Pri 2.0.19 (Orders & Sales) som ťa varoval, že prvé spustenie prepíše obsah stĺpcov Revenue/Profit vo všetkých riadkoch, lebo tam appka zapisuje skutočné vzorce. **Tu sa nič také nedeje.** Dropdown (Data validation) nikdy nemení obsah bunky — len obmedzuje, čo sa dá vybrať, keď na tú bunku klikneš. Existujúce hodnoty v stĺpcoch Platform a Transfer zostanú presne také, aké sú, aj po prvom spustení. Bezpečné kedykoľvek.

## 1. Platform — rastúci dropdown, prepojený s appkou

Presne ako si chcel: "spoj to s dashboradom, ked do dashboradu pridas nove policko tak po update sa to opravi aj v sheete."

Dropdown v stĺpci Platform ukazuje presne tie isté platformy, čo appka ponúka v okne "Add pull" (kde zbieraš pully) — konkrétne tie označené ako nákupné (rovnaká skupina, čo appka už dávno používa aj pri automatickom vytváraní platformy zo sheetu, keď tam napíšeš meno, čo appka ešte nepozná). Keď v appke pridáš novú platformu (cez "+New" pri Pulls), objaví sa v tomto dropdowne od najbližšieho Sync now/Push to sheet/Update sheet. Platí to aj opačne — keď priamo do hárku napíšeš meno platformy, čo appka ešte nepozná, pri najbližšom Sync now ju appka sama založí (toto vlastne appka robí už od úplne prvej verzie Pulls syncu, len doteraz to nebolo vidno ako dropdown).

## 2. Transfer — pevný dropdown Áno/Nie

Presne ako si chcel: "davame 2 moznosti bud ano/nie." V hárku teraz uvidíš na výber presne **Yes** / **No** (appka aj doteraz do tohto stĺpca pri "Push to sheet" zapisovala presne tieto dve anglické slová, takže dropdown len robí viditeľnú tú istú množinu hodnôt, čo appka už predtým používala). Appka pri čítaní hárku naďalej rozumie aj slovenským variantom (Áno/ano/1), tie len nie sú v ponuke dropdownu — keby si niekedy potreboval napísať niečo iné ako Yes/No, appka to prijme, len ti Google Sheets ukáže malú výstrahu (rovnaký princíp "show warning", nie "reject", ako pri Orders & Sales).

## 3. Kedy presne sa to obnoví

Rovnako ako pri Orders & Sales: appka si dropdown skontroluje a podľa potreby doplní automaticky pri **Sync now**, **Push to sheet** aj pri novom tlačidle **Update sheet** (z minulej verzie) — netreba na to nič extra spúšťať. Nový riadok pridaný ručne priamo v hárku dostane fungujúci dropdown hneď (appka nastavuje dropdowny s rezervou dopredu, min. 500 riadkov), aj keď appka o ňom ešte nevie.

## 4. Testy a build

```
cargo test --lib -> 438 passed, 0 failed (bolo 433 - pribudlo 5 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.21 build" v hlavičke)
```

5 nových testov overuje presne rozhodnutia vyššie: že Transfer má vždy presne Yes/No, že Platform ukazuje len platformy označené na nákup (nie predajné), že sa dropdown pre Platform vynechá, keď zatiaľ nemáš žiadnu nákupnú platformu, že chýbajúci stĺpec sa jednoducho preskočí bez chyby, a že žiadny iný stĺpec z tvojho zoznamu (pull/Event name/event date/Ks/More info/Section/Row/Seats/Price) nedostane dropdown.

## 5. Zmenené súbory

**Zmenené:**
- `src-tauri/src/commands/pulls_sheet_sync.rs` — nová `plan_pulls_sheet_structure_updates` (Platform + Transfer) + `ensure_pulls_sheet_structure` + `refresh_pulls_sheet_structure_soft_fail`, zapojené do `sync_pulls`/`push_pulls`/`setup_pulls_sheet`
- `src/pages/Settings.tsx` — popis pri tlačidle "Update sheet" (Pulls karta) doplnený o zmienku, že teraz obnovuje aj dropdowny

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.21`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.21 hotové a overené (438/438 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V hárku Pulls klikni na bunku v stĺpci Platform na hociktorom riadku — mal by sa objaviť dropdown so šípkou, presne s platformami, čo máš v appke označené na nákup.
2. To isté v stĺpci Transfer — dropdown s presne dvoma možnosťami, Yes a No.
3. V appke skús pridať novú platformu (Pulls → Add pull → "+New" pri Platform) a potom spusti Sync now alebo Update sheet — nová platforma by sa mala objaviť v dropdowne v hárku.
4. Skontroluj pár riadkov, čo si mal predtým vyplnené v Platform/Transfer — obsah by mal zostať presne taký, aký bol, len s pribudnutým dropdownom navyše.

Napíš mi, či to takto sedí.
