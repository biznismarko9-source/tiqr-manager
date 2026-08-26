# TIQR Manager 2.0.53 — Convert to EUR sa teraz pošle aj do tvojho Google Sheetu

## Čo si mi napísal

*"po converte ked dam sync sheets a sales tak tiez by sa ta mena mala zmenit v tabulke a stlpec currency tiez sa zmenit na EUR"*

Po tom, čo som ti vysvetlil, čo presne som našiel (nižšie), si potvrdil: áno, presne toto - chceš, aby sa prevod poslal aj do Sheetu.

## Čo som našiel

`Convert to EUR` (2.0.51) vždy správne prepočíta a uloží EUR do objednávky, každého lístka aj každého predaja - v appke samotnej je teda po prevode všetko konzistentné hneď. Problém je inde: synchronizácia objednávok s Google Sheets je zámerne postavená tak, že objednávka, ktorá už raz prešla synchronizáciou (má svoje "TIQR ID"), sa **odvtedy nikdy viac neupravuje ani jedným smerom** - ani sťahovaním zo Sheetu, ani posielaním do neho. Dôvod je vážny: úprava objednávky po tom, čo už existujú lístky, by sa dotýkala presného výpočtu nákladov na cent, čo je citlivá finančná logika, na ktorú appka schválne nesiaha bez opýtania. Výsledok: prevod na EUR sa v appke uloží správne, ale tvoj skutočný Sheet sa to nikdy nedozvie - riadok tam navždy ostane so starou menou aj sumami.

## Čo je nové

Po úspešnom prevode (jednotlivá objednávka cez Order Detail, aj hromadný prevod z Dashboardu) appka teraz **navyše** skúsi aktualizovať presne 3 bunky v prepojenom riadku Sheetu - **Currency**, **Price Per Ticket**, **Total Purchase Price** - na nové, prevedené hodnoty.

Toto je zámerne úzka, jasne ohraničená výnimka z pravidla vyššie, nie jeho zrušenie:
- Nič iné o objednávke sa takto nemení (množstvo, platforma, dátum, ...) - žiadna nová cesta na úpravu, žiadne nové porovnávanie bunka po bunke, presne to, čo pravidlo chránilo, ostáva chránené.
- Ak objednávka NIKDY nebola so Sheetom prepojená (bežný prípad - ručne zadané alebo cez CSV), nič sa ani neskúša, úplne ticho.
- Ak sa samotný prevod v appke podarí, ale zápis do Sheetu zlyhá (výpadok siete, oprávnenia, riadok medzitým zmizol...), appka ti to jasne napíše v tej istej hláške - nikdy to nezamlčí. Prevod samotný pritom zostáva plne uložený bez ohľadu na to, či sa zápis do Sheetu podaril.
- Pri hromadnom prevode z Dashboardu appka spočíta, koľko prevedených objednávok bolo vôbec so Sheetom prepojených a koľko z nich sa podarilo/nepodarilo zapísať, a napíše to ako jedno súhrnné číslo namiesto zoznamu pre každú objednávku zvlášť.

## Ako som to overoval

Rovnaké obmedzenie ako pri 2.0.52 - žiadny `rustc`/`cargo`/`npx tsc -b`/`npm run build` k dispozícii v tomto prostredí, takže znova staticky:

- Nová logika je rozdelená na čistú, testovateľnú časť (ktoré bunky treba zapísať, podľa už stiahnutých dát zo Sheetu) a tenkú "sieťovú" časť (skutočné volania na Google API) - presne ten istý vzor, aký appka už používa pri `push_sales`. Vďaka tomu som mohol napísať **7 nových testov** pre tú prvú časť bez potreby reálneho pripojenia (nájde správny riadok podľa markera, vráti nič keď marker chýba/stĺpec chýba, zapíše aspoň to, čo sa dá, keď jeden stĺpec v Sheete chýba, a 3 testy na to, že appka zlyhá zrozumiteľne - nie ticho a nie pádom - keď objednávka nie je prepojená / Sheet nie je pripojený / chýba prihlásenie).
- `Cargo.toml`/`Cargo.lock` sa v tomto kole vôbec nemenili (žiadna nová závislosť).
- Skontroloval som naschvál aj to, že nové polia (`linkedToSheet`, `sheetPushError`) majú na Rust aj TypeScript strane rovnaké mená (appka mení `snake_case` na `camelCase` automaticky, ale len ak je na to na Rust strane pripravená - overil som, že je) - presne tá trieda chyby, čo by inak spôsobila, že appka by tíško dostávala `undefined` namiesto skutočnej hodnoty.
- Vlastný skript na kontrolu spárovania zátvoriek (rovnaký ako pri 2.0.52) prešiel čisto na všetkých troch upravených `.rs` súboroch.
- Drobnosť na záver: pri kontrole `package-lock.json` som si všimol, že nezávislý balíček `node-releases` je (celkom náhodou) tiež na verzii 2.0.53 - overil som, že to nie je omyl z môjho úpravovania (bol na tejto verzii už predtým, nikdy nebol na "2.0.52", takže sa ho môj príkaz na zmenu verzie vôbec nedotkol), len zhoda čísel, nič viac.

## Čo teraz urobiť

1. Nainštaluj 2.0.53.
2. Nájdi objednávku, ktorá prišla zo synchronizácie so Sheetom a nie je v EUR (alebo takú vytvor/synchronizuj nanovo).
3. V Order Detail klikni **Convert to EUR**.
4. Otvor svoj skutočný Google Sheet a skontroluj riadok tej objednávky - stĺpce Currency, Price Per Ticket aj Total Purchase Price by mali byť teraz aktualizované na EUR a prepočítané sumy.
5. Ak máš viac takých objednávok, skús aj hromadný prevod na Dashboarde a over to isté na viacerých riadkoch naraz.

## Zmenené súbory

**Backend (Rust, 3 súbory):**
- `src-tauri/src/commands/orders_sheet_sync.rs` - nová logika na zápis do Sheetu (`currency_push_cells` + `push_order_currency_to_sheet`) + 7 nových testov
- `src-tauri/src/commands/orders.rs` - zavolanie tejto logiky po úspešnom prevode (jednotlivom aj hromadnom)
- `src-tauri/src/models.rs` - 2 nové polia na `OrderCurrencyConversion` (`linked_to_sheet`, `sheet_push_error`)

**Frontend (3 súbory):**
- `src/lib/types.ts` - rovnaké 2 nové polia
- `src/pages/OrderDetail.tsx` - hláška po prevode teraz spomenie aj výsledok zápisu do Sheetu
- `src/pages/Dashboard.tsx` - to isté, súhrnne pre hromadný prevod

**Verzia (8 miest):** ako vždy, všetkých na `2.0.53`.

## Čo sa NEMENILO

Žiadna migrácia. Order sync pre všetko OKREM meny (množstvo, platforma, dátum, sekcia/rad/sedadlo, ...) je stále presne tak "append-only, nikdy sa nedotkni znova" ako predtým - toto kolo pridáva jednu úzku výnimku, nie všeobecnú cestu na úpravy. `push_sales`/Sales sync sa vôbec nemenili.

## STOP

2.0.53 hotové. Skontroluj prosím presne podľa krokov vyššie - najdôležitejšie je overiť priamo v tvojom Google Sheete, že sa tam čísla naozaj zmenili, keďže to je jediná časť, ktorú som v tomto prostredí nemohol vyskúšať naživo.
