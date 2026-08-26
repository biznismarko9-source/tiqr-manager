# TIQR Manager 2.0.25 — Pull sa vypĺňa pri vytváraní objednávky

## Čo je nové

Vo formulári **"New order"** (Orders → New order), v sekcii Purchase, hneď pod "Other costs", je nová zaškrtávacia možnosť:

> ☐ **This order was pulled by someone else**

Keď ju zaškrtneš, zobrazia sa 2 polia — **Who pulled** (kto to pulol) a **Pull fee** (koľko si mu zaplatil). Event, dátum, počet kusov a mena sa nikam nepíšu znova — appka ich pozná z objednávky, ktorú práve vytváraš.

Presne podľa tvojho príkladu — **pull fee sa pripočíta k celkovej nákupnej cene**. Kúpiš lístky za 200, pull ťa stál 20, v súhrne dole vo formulári aj v samotnej objednávke bude Total = 220. V súhrne pribudol aj nový riadok "Pull fee", nech vidíš presne, koľko sa pripočítalo.

Po uložení objednávky appka **automaticky vytvorí aj záznam v Received Pulls**, prepojený na túto novú objednávku — presne to, čo si chcel: zaškrtneš, vyplníš, a je to v pulloch, bez ďalšieho kroku.

## ⚠️ Čo som si musel domyslieť — over si to

1. **Sekcia "Received pulls" na Order Detail (z minulej verzie 2.0.24) som nechal tak, ako bola** — teraz je to teda "aj aj": pri vytváraní vieš zaškrtnúť rovno, a na Order Detail to vieš kedykoľvek doplniť/upraviť neskôr (napr. ak si na to pri vytváraní zabudol, alebo chceš pridať druhý pull k tej istej objednávke). Ak by si chcel, aby na Order Detail táto sekcia úplne zmizla a pull sa dal robiť LEN pri vytváraní, napíš mi a odstránim ju.

2. **Ako presne funguje "pripočíta sa k cene"** — toto je citlivá finančná logika, tak vysvetľujem presne: appka už dávno mala políčko "Other costs (total)" — jedna suma za celú objednávku, ktorá sa rovnomerne rozpočíta na všetky lístky (rovnaký mechanizmus, čo používajú aj "Fees"). Pull fee som **pripočítal k tejto istej sume** — nie je to nové políčko v databáze, len sa k tomu, čo napíšeš do "Other costs", pripočíta aj pull fee, a spolu sa to pošle appke presne tak, ako doteraz. Vďaka tomu som sa vôbec nemusel dotknúť toho, ako appka rozpočítava náklady na jednotlivé lístky (to je chránená logika, ktorú nemením bez opýtania) — len som zmenil, AKÁ suma sa do toho existujúceho výpočtu pošle.
   - Dôsledok: pull fee sa (rovnako ako "Other costs") **rozpočíta rovnomerne na všetky lístky** tej objednávky. Pri 200+20 na 1 lístku je to jasné, ale napr. pri 4 lístkoch by sa pull fee 20 rozpočítalo po 5 na každý lístok, presne tak, ako sa dnes správa "Other costs".
   - Dôsledok č. 2: po vytvorení objednávky sa pull fee **už nezobrazuje ako samostatná položka** v karte "Fees + other" na Order Detail (splynie s Other costs) — samotný záznam v Received Pulls si ale presnú sumu aj meno pamätá zvlášť, takže informácia sa nestratí, len na Order Detail nevidno "pull fee" ako vlastný riadok po uložení.
   
   Ak by si chcel, aby to bolo inak (napr. pull fee zostal navždy viditeľný ako vlastná položka, nie zlúčený do Other costs), daj vedieť — dá sa to spraviť, len by to vyžadovalo väčší zásah (nové pole v databáze).

3. **Text tlačidla/otázky** — dal som "This order was pulled by someone else" namiesto tvojho pracovného návrhu "did you buy pull", nech to znie prirodzenejšie v kontexte formulára. Ak chceš iné znenie, napíš aké.

## Testy a build

Toto je čisto frontendová zmena (formulár New order) — v Rust kóde som nemenil nič, znovu som len použil už hotový a otestovaný príkaz z 2.0.24 (`link_pull_received_to_order`).

```
cargo test --lib -> 470 passed, 0 failed (bez zmeny - žiadny Rust kód sa nemenil)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.25 build" v hlavičke)
```

## Zmenené súbory

**Zmenené:**
- `src/pages/Orders.tsx` — nový checkbox "This order was pulled by someone else" + polia Who pulled/Pull fee vo formulári New order, pull fee sa pripočíta do `otherCostsCents`, po vytvorení objednávky sa zavolá prepojenie na Received Pulls

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.25`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.25 hotové a overené (470/470 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj presne tvoj príklad:

1. Orders → New order. Vyplň lístky za 200 (napr. 1 ks po 200), zaškrtni "This order was pulled by someone else", vyplň meno a pull fee 20.
2. V súhrne dole skontroluj: Purchase 200, Pull fee 20, Total 220.
3. Ulož → skontroluj, že objednávka má Total cost 220 (Order Detail), a že v Pulls → Received Pulls pribudol nový záznam s menom a sumou 20, prepojený na túto objednávku.
4. Skontroluj bod ⚠️ č. 2 vyššie — hlavne, či ti vyhovuje, že sa pull fee rozpočíta rovnomerne na lístky (ako Other costs) a že po uložení splynie s "Other costs" v zobrazení.
