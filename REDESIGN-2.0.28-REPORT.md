# TIQR Manager 2.0.28 — Hromadné mazanie: Pulls, Orders, Events aj Sales

## Čo je nové

Presne podľa zadania — na **Pulls (Given aj Received), Orders, Events a Sales** je teraz možné zmazať viac položiek naraz, presne tým postupom, aký si opísal:

- Bežne je zoznam úplne čistý — žiadny checkbox stĺpec navyše, nič, čo by tam predtým nebolo.
- Vedľa existujúceho hlavného tlačidla (napr. "+ New Event") pribudlo druhé, menšie tlačidlo **"Delete"**.
- Klikneš naň → pri každom riadku sa objaví checkbox a nad tabuľkou sa objaví červený pruh s tým, koľko máš vybraných, tlačidlom "Delete selected" a odkazom "Cancel".
- Vyberieš, čo chceš zmazať (fajočkou pri riadku, alebo klikom kdekoľvek na riadok — funguje oboje), klikneš "Delete selected", appka sa ešte raz opýta (rovnaké potvrdzovacie okienko, aké appka používa všade inde pri mazaní), a až po potvrdení sa to naozaj zmaže.
- Hneď po zmazaní (alebo po kliku na Cancel) checkboxy aj červený pruh zmiznú. Znova sa objavia až vtedy, keď znova klikneš na "Delete" — presne ako si chcel, nič nezostáva "nastálo zapnuté".

Bezpečnostné pravidlá zostali presne také, aké appka už mala pri mazaní po jednom: objednávku s predanými lístkami alebo akoukoľvek históriou predaja (vrátane refundov) appka nezmaže, event s naviazanými objednávkami/lístkami tiež nie — pri hromadnom mazaní to funguje rovnako, len namiesto toho, aby to celé zablokovalo, appka **tú jednu nebezpečnú položku len preskočí** (a napíše prečo), a všetko ostatné z výberu pokojne zmaže. Takže napríklad keď v Orders vyberieš 10 objednávok a 2 z nich majú predané lístky, zmaže sa 8 a dostaneš správu, čo presne sa preskočilo a prečo — nikdy sa nestane, že by ti kvôli jednej "zamknutej" položke appka odmietla zmazať aj tie ostatné. Pulls a Sales žiadne takéto obmedzenie nemajú (tak to funguje aj pri mazaní po jednom už dnes), takže tam sa vždy zmaže presne to, čo vyberieš.

Po zmazaní appka ukáže zelenú správu s počtom zmazaných položiek, a ak sa niečo preskočilo, aj červenú správu s dôvodom — keď appka preskočí viac položiek z rovnakého dôvodu, napíše to raz s počtom (napr. "2x This order has sold tickets...") namiesto toho, aby opakovala tú istú vetu viackrát za sebou.

## Ako presne to funguje pod kapotou

Hromadné mazanie pri Orders aj Events **nevymýšľa žiadne nové pravidlo** — interne volá presne tú istú kontrolu, akú appka už dnes robí pri mazaní jednej objednávky/eventu (vytiahol som ju do samostatnej zdieľanej funkcie, aby ju používali obe cesty naraz), takže hromadné mazanie nemôže nikdy dovoliť niečo, čo by appka pri mazaní po jednom odmietla.

Technicky je to zámerne inak, ako appka bežne robí hromadné operácie (typicky: najprv si overí úplne všetko, a až potom to celé naraz zapíše — buď sa podarí všetko, alebo nič). Pri mazaní som sa rozhodol pre iný, bezpečnejší prístup: appka posúdi každú vybranú položku samostatne — všetko, čo je v poriadku zmazať, sa zmaže naraz v jednej databázovej transakcii (takže to nie je pomalé "jedno po druhom" a ani prípadný pád appky uprostred procesu nemôže nechať polovicu zmazanú), ale jedna nebezpečná položka vo výbere nikdy nezastaví zmazanie tých ostatných. Pri niečom nezvratnom, ako je mazanie, mi to prišlo ako jednoznačne správnejšie správanie než "všetko alebo nič" — ak si s týmto nesúhlasil, napíš mi a viem to prerobiť.

Na frontende je to jedna zdieľaná komponenta (`BulkDeleteBar`) použitá na všetkých 4 obrazovkách namiesto toho, aby mala každá vlastnú - vizuálne je to založené na tom istom červenom pruhu, aký už appka má na Sale Detail pri hromadných akciách s lístkami. Každá stránka si sama pamätá, či je práve v "režime výberu" - a presne to je to, čo checkboxy zapína/vypína.

`events.rs` mal doteraz zámerne bez vlastných testov (viď poznámka v `PROTECTED-AREAS-NOTES.md`) - pre novú funkciu mazania som mu pridal jeho vôbec prvý testovací súbor, ale výhradne pre túto novú logiku, zvyšok súboru sa netestuje ďalej, presne podľa toho, ako to bolo doteraz zdôvodnené.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bolo 475 - pribudlo 16 nových testov na 5 nových príkazov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.28 build" v hlavičke)
```

Naviac som si všetko aj vizuálne overil cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) - na všetkých 4 obrazovkách (Pulls v oboch záložkách Given aj Received), svetlý aj tmavý režim: čistý zoznam, zapnutie výberu, výber vrátane úmyselne "zamknutej" položky, potvrdzovacie okienko, výsledok po zmazaní s oboma typmi správ (zmazané aj preskočené - vrátane scenára s dvoma rôznymi dôvodmi preskočenia naraz pri Orders), zrušenie cez Cancel bez toho, aby sa čokoľvek zmazalo, a klik na riadok (nie na checkbox) na Sales, ktorý predtým žiadnu vlastnú akciu pri kliku nemal.

## Zmenené súbory

**Backend:**
- `src-tauri/src/models.rs` - nové zdieľané typy `BulkDeleteResult`/`BulkDeleteSkip`
- `src-tauri/src/commands/pulls.rs`, `commands/pulls_received.rs` - nový `bulk_delete_*` príkaz + testy
- `src-tauri/src/commands/orders.rs` - vytiahnutá zdieľaná `order_delete_blocker`, nový `bulk_delete_orders` + testy
- `src-tauri/src/commands/events.rs` - vytiahnutá zdieľaná `event_delete_blocker`, nový `bulk_delete_events` + jeho prvý testovací súbor
- `src-tauri/src/commands/sales.rs` - vytiahnutá zdieľaná `delete_sale_group_rows`, nový `bulk_delete_sale_groups` + testy
- `src-tauri/src/lib.rs` - registrácia 5 nových príkazov

**Frontend:**
- `src/lib/types.ts` - nový typ `BulkDeleteResult`
- `src/lib/api.ts` - 5 nových volaní (`bulkDeleteEvents`/`bulkDeleteOrders`/`bulkDeletePulls`/`bulkDeletePullsReceived`/`bulkDeleteSaleGroups`)
- `src/lib/format.ts` - `summarizeBulkDeleteSkips` (zoskupenie preskočených položiek podľa dôvodu)
- `src/components/ui.tsx` - nová zdieľaná komponenta `BulkDeleteBar`
- `src/pages/Events.tsx`, `src/pages/Orders.tsx`, `src/pages/Sales.tsx`, `src/pages/Pulls.tsx` - tlačidlo "Delete", výber, potvrdenie, mazanie na každej obrazovke (Pulls: obe záložky, Given aj Received, samostatne)

**Verzia (7 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` - všetkých 7 na `2.0.28`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.28 hotové a overené (491/491 testov, čisté `tsc`/`build`, vizuálne overené). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Na každej zo 4 obrazoviek (Pulls má dve záložky - Given aj Received, skús obe) klikni "Delete", vyber pár položiek, klikni "Delete selected", potvrď - zmaže sa presne to, čo si vybral, a checkboxy zase zmiznú.
2. V Orders alebo Events skús do výberu zahrnúť aj niečo, čo má predané lístky / históriu predaja (resp. naviazané objednávky) - over si, že sa to preskočí a appka ti napíše prečo, namiesto toho aby to celé zlyhalo.
3. Klikni "Delete", niečo vyber, ale namiesto potvrdenia klikni "Cancel" - over, že sa nič nezmazalo. Klikni "Delete" znova - over, že checkboxy sú znova prázdne, appka si nepamätá predchádzajúci výber.
4. Ak ti nevyhovuje, že sa nebezpečná položka len preskočí namiesto toho, aby zablokovala celý výber (vysvetlené vyššie) - napíš mi, viem to prerobiť na "všetko alebo nič".

## Dodatok (23.8.2026) — oprava balenia ZIP-u

Prvý ZIP, čo som k tejto verzii poslal, mal chybu **v balení, nie vo funkcii vyššie**: chýbali mu dva súbory (`tsconfig.node.json`, `vite.config.ts`), ktoré `tsc` potrebuje na build. Preto GitHub Actions build v CI zlyhal hneď na začiatku a nevznikol žiadny inštalátor. Popísané v `PROTECTED-AREAS-NOTES.md`. Opravený ZIP (rovnaká verzia 2.0.28, keďže tá prvá sa nikdy naozaj nevydala) som tentokrát overil aj tak, že som ho vybalil úplne samostatne mimo môjho pracovného priečinka a spustil build len z toho - presne to, čo predtým chýbalo v mojom postupe. Samotná funkcia hromadného mazania (celý zvyšok tohto reportu) touto chybou nebola nijak dotknutá.
