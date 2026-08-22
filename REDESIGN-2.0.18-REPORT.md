# TIQR Manager 2.0.18 — Push to sheet (appka → hárok)

Presne to, čo si chcel: doteraz sync fungoval len jedným smerom — **hárok → appka**. Teraz pribudol aj druhý smer — **appka → hárok** — pre všetky tri veci naraz, presne ako si vybral ("Všetky tri naraz"): **Pulls**, **Orders** aj **Sales**.

Na každej pripojenej karte v Settings → Integrations je teraz vedľa existujúceho tlačidla na sync nové tlačidlo na push — **dve samostatné tlačidlá**, presne ako si vybral ("Dve tlačidlá"), nie jedno, čo by robilo oboje naraz. Nič sa nedeje samo — push sa spustí len vtedy, keď naň klikneš.

## 1. Pulls — "Push to sheet"

Na karte **Pulls** je teraz vedľa "Sync now" aj tlačidlo **"Push to sheet"**:

- Nový pull, čo si zapísal priamo v appke (a ešte nikdy nebol v hárku), sa pridá ako **nový riadok** na koniec hárku — presne ako si vybral ("Áno, pridať riadok").
- Pull, čo už raz prišiel/odišiel cez sync a ty si ho **upravil v appke**, sa zapíše späť do hárku — ale **bunku po bunke**, nie celý riadok naraz, aby appka nikdy nezasiahla nič, čo do toho riadku prípadne dopisuješ ty sám mimo týchto stĺpcov.
- Bezpečnostná poistka: ak sa ten istý riadok medzičasom **zmenil aj priamo v hárku** (napr. si niečo prepísal ručne v Google Sheets), appka to **nepretlačí ticho navrch** — nahlási ti to ako konflikt a povie ti, aby si najprv spravil "Sync now" (aby appka videla tvoju ručnú zmenu) a až potom skúsil push znova. Inak by sa mohlo stať, že appka prepíše niečo, čo si práve zmenil v hárku, a ty by si o tom ani nevedel.

## 2. Orders — "Push orders" (append-only)

Na karte **Orders & Sales** je vedľa "Order sync"/"Sales sync" teraz aj **"Push orders"**. Tu je dôležité obmedzenie, ktoré si prosím pozorne prečítaj:

- Nová objednávka, čo si vytvoril priamo v appke (nie cez sync z hárku), sa pridá ako **nový riadok** do hárku.
- Objednávka, čo appka **už raz videla** (bola vytvorená cez sync z hárku, alebo bola už predtým pushnutá), sa **už nikdy znova neupravuje** — ani keby si jej nákupné údaje neskôr zmenil v appke.

**Prečo takto:** náklady na objednávku sa v appke v momente vytvorenia rozpočítajú presne (do centu) na jednotlivé lístky — to je citlivý výpočet, ktorého sa táto appka nesmie dotýkať bez tvojho vedomého zásahu. Keby push mohol neskôr prepisovať cenu/množstvo už vytvorenej objednávky v hárku, riskovalo by to, že sa táto matematika niekedy rozíde s tým, čo appka reálne eviduje. Ak potrebuješ opraviť už zosynchronizovanú objednávku, urob to ručne priamo v hárku alebo v appke — push sa jej proste nedotkne, ani v jednom smere.

## 3. Sales — "Push sales" (len do prázdnych buniek)

Vedľa "Push orders" je aj **"Push sales"**. Toto je najopatrnejšia z celej trojice, zámerne:

- Appka doplní stĺpce **Site Listed / Payout Per Ticket / Status / Delivery status / Payout status / dátum predaja / paid by** na riadok objednávky, ktorú appka pozná — **ale len vtedy, keď sú v hárku VŠETKY tieto bunky na danom riadku úplne prázdne.** Ak je v hárku čo i len jedna z nich už niečím vyplnená (nezáleží, či sedí alebo nesedí s appkou), appka **celý ten riadok nechá tak** — nič neprepisuje, ani nehlási chybu.
- Appka to urobí len vtedy, keď je objednávka **celá a rovnako predaná** — teda každý lístok tej objednávky má presne tú istú platformu, cenu, stav platby, dátum predaja aj kupujúceho. Ak si časť lístkov predal inak než zvyšok (napr. po kusoch, za rôzne ceny), appka to považuje za "nejednotný predaj" a radšej nič nedopĺňa, než aby do jedného riadku vpísala len jednu z viacerých rôznych cien.
- Nezávisle od tohto appka rovnako doplní aj stĺpce **pull / who pulled / how much pull** — ale len vtedy, keď je na tú objednávku napojený **presne jeden** Received pull (ak nie je napojený žiadny, alebo sú napojené dva, appka to radšej nechá na teba).

**Prečo takto:** tvoje stĺpce **Revenue** a **Profit** v hárku sú vzorce, ktoré appka nikdy nečíta ani nezapisuje — tie sú celé tvoje. A keďže appka zapisuje po jednotlivých bunkách (nie po celých riadkoch), nemôže sa stať, že by push omylom prepísal susedný stĺpec, čo appka vôbec nepozná.

## 4. "Last pushed" — samostatný časový údaj

Na každej karte pribudol riadok **"Last pushed: ..."** vedľa existujúceho "Last synced: ...". Sú to dva úplne nezávislé časové údaje — sync a push sú dva rôzne smery, takže appka si aj čas ich posledného spustenia pamätá oddelene.

## 5. Zhrnutie — čo appka nikdy neurobí

- Nikdy nepretlačí zmenu do hárku, ak sa riadok medzičasom zmenil aj tam (Pulls) — nahlási konflikt namiesto tichého prepisu.
- Nikdy neupraví objednávku, ktorú už raz videla (Orders) — len pridáva nové riadky.
- Nikdy neprepíše bunku, v ktorej už niečo je (Sales) — dopĺňa len úplne prázdne bunky.
- Nikdy sa nedotkne stĺpcov Revenue/Profit ani žiadneho iného stĺpca, ktorý appka nepozná.
- Testovacie/demo dáta sa nikdy nepushnú do tvojho skutočného hárku.

## 6. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 404 passed, 0 failed (bolo 376 - pribudlo 28 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.18 build" v hlavičke)
```

28 nových testov pokrýva presne rozhodnutia vyššie: že nový pull/objednávka sa pridá ako nový riadok a hneď sa napojí, že demo dáta sa nikdy nepushnú, že úprava už napojeného pullu v appke zapíše zmeny bunka po bunke, že zmena v hárku od posledného sync-u zablokuje push a nahlási konflikt, že vymazaný riadok v hárku sa nikdy ticho znova nepridá, že už napojená objednávka sa pri push-i nikdy nedotkne, že nejednotný predaj naprieč lístkami jednej objednávky sa nechá tak, že riadok s čo i len jednou vyplnenou bunkou v cieľovej skupine sa nechá úplne netknutý, že pull stĺpce sa doplnia len pri presne jednom napojenom Received pulle, a že "Last pushed" sa sleduje nezávisle od "Last synced".

## 7. Zmenené súbory

**Zmenené:**
- `src-tauri/src/commands/pulls_sheet_sync.rs` — nová funkcia `push_pulls` (append/update/konflikt), refaktor `parse_pull_row` do samostatnej funkcie
- `src-tauri/src/commands/orders_sheet_sync.rs` — nové funkcie `push_orders` (append-only) a `push_sales` (len do prázdnych buniek, vrátane napojenia na Received pulls)
- `src-tauri/src/commands/sheets_sync.rs` — nový nezávislý časový údaj `last_pushed_at`
- `src-tauri/src/models.rs` — `SheetsConnectionStatus` má nové pole `last_pushed_at`
- `src-tauri/src/google_sheets.rs` — `append_values` teraz reálne používaná (predtým pripravená, nepoužitá)
- `src-tauri/src/lib.rs` — zaregistrované nové príkazy `push_pulls`, `push_orders`, `push_sales`
- `src/lib/api.ts`, `src/lib/types.ts` — nové volania a typ `lastPushedAt`
- `src/pages/Settings.tsx` — každá karta má teraz aj tlačidlo na push, vlastný výsledok a "Last pushed" riadok

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.18`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.18 hotové a overené (404/404 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Na kartách Pulls aj Orders & Sales v Settings → Integrations vidíš nové tlačidlá ("Push to sheet" / "Push orders" / "Push sales") vedľa existujúcich sync tlačidiel.
2. Skús v appke ručne pridať nový pull (alebo objednávku) a klikni na push — mal by pribudnúť nový riadok na konci tvojho hárku.
3. Skús v appke upraviť už zosynchronizovaný pull a znova klikni "Push to sheet" — zmena by sa mala prejaviť v príslušných bunkách toho riadku v hárku.
4. Skús predať všetky lístky jednej objednávky rovnako (rovnaká platforma/cena/dátum) a klikni "Push sales" — ak sú príslušné bunky v hárku prázdne, mali by sa doplniť.

Napíš mi, či to takto sedí, alebo či niečo chceš inak (napr. iné správanie pri konflikte, alebo aby Orders push predsa len vedel niečo dopĺňať aj na už zosynchronizovanú objednávku).
