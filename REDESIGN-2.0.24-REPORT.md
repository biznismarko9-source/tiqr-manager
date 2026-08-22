# TIQR Manager 2.0.24 — Pull info priamo pri objednávke v appke

## Čo je nové

Na obrazovke **Order Detail** (klikneš na objednávku v Orders) je teraz nová sekcia **"Received pulls"**, hneď pod Platform/Notes/Currency. Funguje presne tak, ako si chcel — nie cez hárok, priamo v appke:

- Tlačidlo **"Add pull info"** otvorí malé okienko s 2 políčkami: **kto to pulol** a **koľko si mu zaplatil**.
- Event, dátum, počet kusov a mena sa **automaticky doplnia z tej istej objednávky** — nepíšeš ich znova, appka ich už pozná.
- Po uložení appka **automaticky vytvorí záznam v Received Pulls** presne tak, ako to už dnes robí synchronizácia z hárku — len teraz to vieš spraviť aj bez hárku, priamo tu.
- Ak je k objednávke už niečo prepojené, vidíš to hneď v tej istej sekcii (meno, počet, suma) a kliknutím to vieš plnohodnotne upraviť (aj zmazať) — otvorí sa ten istý formulár, čo poznáš z Pulls → Received Pulls.

## ⚠️ Rozhodnutia, ktoré som urobil sám — over si ich

1. **Kde presne som to pridal**: na **Order Detail** (detail konkrétnej objednávky), nie do formulára "New order" pri jej vytváraní. Dôvod: presne tak je to umiestnené aj v hárku — `pull`/`who pulled`/`how much pull` sú tam v druhej skupine stĺpcov (tá, čo sa vypĺňa neskôr, pri predaji), nie v prvej (nákupnej). Ak by si to chcel mať aj rovno pri zakladaní novej objednávky, daj vedieť — pridám to aj tam.

2. **Zjednodušil som z 3 polí na 2**: v hárku sú 3 stĺpce (`pull` Yes/No, `who pulled`, `how much pull`), lebo bunka v Sheete potrebuje vedieť rozlíšiť "zámerne nie" od "ešte nevyplnené". V appke to takýto rozdiel nepotrebuje — stačí vyplniť meno toho, kto to pulol, a to JE to "áno". Takže v appke je to len **"Who pulled"** + **"How much"**, žiadne zvlášť "Pull: Áno/Nie". Funkčne to vyjde na to isté.

3. **Viacero pullov na jednu objednávku**: appka to nijako neobmedzuje (rovnako ako doteraz na obrazovke Pulls, kde si tiež mohol prepojiť ľubovoľnú objednávku bez obmedzenia). Ak by si omylom klikol "Add pull info" dvakrát, vzniknú dva záznamy — nič ťa nezastaví, presne ako doteraz na Pulls.

4. **Suma je nepovinná** — necháš ju prázdnu, appka ju uloží ako 0 (rovnaký princíp ako pri hárku: "how much pull" nikdy nič neblokuje).

## Ako presne to funguje pod kapotou

Appka pridala 2 nové príkazy:
- **Add pull info** → vytvorí nový, ručne pridaný záznam v Received Pulls (rovnaký typ záznamu, ako keď si ho vytvoril na obrazovke Pulls) a rovno ho prepojí na túto objednávku.
- Editovanie už prepojeného pullu **znovupoužíva presne ten istý formulár**, čo poznáš z Pulls → Received Pulls (nie je to duplicitná appka v appke) — tam si vieš opraviť aj Event/dátum/počet/menu, ak by bolo niečo zle.

Toto je úplne nezávislé od hárku a od Google Sheets synchronizácie — obe cesty (hárok aj priamo appka) teraz vedú do toho istého miesta (Received Pulls) a fungujú vedľa seba.

## Testy a build

```
cargo test --lib -> 470 passed, 0 failed (bolo 461 - pribudlo 9 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.24 build" v hlavičke)
```

## Zmenené súbory

**Zmenené:**
- `src-tauri/src/commands/pulls_received.rs` — nová `link_pull_received_to_order` (Add pull info) a `list_pulls_received_for_order` (zobrazenie v Order Detail)
- `src-tauri/src/lib.rs` — zaregistrované 2 nové príkazy
- `src/pages/OrderDetail.tsx` — nová sekcia "Received pulls" + jednoduché okienko "Add pull info"
- `src/pages/Pulls.tsx` — `PullReceivedFormModal` je teraz exportovaný, aby ho Order Detail vedel znovupoužiť na editáciu
- `src/lib/api.ts` — 2 nové funkcie (`linkPullReceivedToOrder`, `listPullsReceivedForOrder`)

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.24`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.24 hotové a overené (470/470 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Otvor ľubovoľnú objednávku (Orders → klikni na riadok) → mala by tam byť nová sekcia "Received pulls" pod Platform/Notes/Currency kartou.
2. Klikni "Add pull info", vyplň meno a sumu, ulož → mal by pribudnúť riadok priamo v tejto sekcii, aj v Pulls → Received Pulls (skontroluj, že Event/dátum/počet/mena sedia s objednávkou).
3. Klikni na ten pridaný riadok → mal by sa otvoriť plný formulár na úpravu (rovnaký, čo poznáš z Pulls).
4. Skontroluj bod ⚠️ vyššie — hlavne či ti sedí, že som to pridal len na Order Detail (nie aj pri zakladaní novej objednávky), a že som z 3 polí spravil 2.
