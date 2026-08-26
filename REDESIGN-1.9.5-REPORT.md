# TIQR Manager 1.9.5 — Sales Platform filter, Settings Data layout, Tickets Order link

Report k verzii **1.9.5**. Nadväzuje na 1.9.4. Z tvojej spätnej väzby k trom screenshotom (Sales, Settings
→ Data, Dashboard) som implementoval 3 veci a **pri dvoch som sa radšej opýtal**, než aby som hádal - nižšie
presne prečo. **Žiadna zmena v `src-tauri/` - opäť celé len frontend**, overené na súborovom systéme
(sekcia 6).

---

## 1. Sales — Platform filter zúžený na sale/both

**Čo si napísal:** pri Sales sú stále všetky platformy, tam by mali byť platformy, kde sa predáva.

Toto je priama zmena rozhodnutia z 1.9.3/1.9.4, kde Sales filter zostal **zámerne** nezúžený (dôvod vtedy:
vyhľadávanie naprieč všetkými historickými predajmi, nie výber platformy pre jeden konkrétny predaj). Teraz
si sa naň pozrel znova a chceš to inak - rešpektujem to, žiadna otázka netreba, je to jednoznačné. Platform
filter na Sales teraz zobrazuje len `kind === "sale" || kind === "both"` - presne rovnaký vzor, aký majú
Orders/Order Detail (`purchase`/`both`) a od 1.9.4 aj Tickets/Inventory filter. Teraz je **každý** Platform
picker v appke takto zúžený, žiadna výnimka neostáva. Aktualizoval som aj krížové komentáre v `Sales.tsx`
a `Tickets.tsx`, ktoré predtým na seba odkazovali ako na "ten, čo je iný" - už nie sú iné.

---

## 2. Settings → Data — karty pod sebou

**Čo si napísal:** v Nastaveniach → Data by si to dal pod seba.

Predtým: `grid-cols-1 lg:grid-cols-2` - Import a Export vedľa seba na širších oknách, Backup & restore pod
nimi cez celú šírku. Teraz: jednoduché `grid-cols-1` bez responsívnej výnimky - všetky tri karty (Import,
Export, Backup & restore) idú vždy pod sebou, nezávisle od šírky okna. Obsah kariet samotný je nezmenený.

---

## 3. Tickets/Inventory — Order odkaz vždy funkčný

**Čo si napísal a čo som z toho spravil:** napísal si, že na Tickets sa dá preklikať len na Order, nie na
Event, a hneď potom si opísal vzor, ktorý majú Event/Orders/Sales - že sa dá kliknúť na vlastný záznam - a
chcel si to isté aj v Tickets a Inventory.

**Čo som k tomu zistil pri overovaní kódu:** Order aj Event bunka v tabuľke používali **presne ten istý**
`allowCrossLinks` prepínač - a ten je dnes zapnutý na oboch stránkach (Tickets aj Inventory). Nenašiel som
v kóde žiadnu asymetriu, ktorá by vysvetľovala, prečo by Order fungoval a Event nie - obe by sa mali
správať identicky na 1.9.4. Možné vysvetlenie: appka, na ktorej si to skúšal, ešte nemala nainštalovanú
1.9.4 (tá išla len pred chvíľou) - `1-CLICK-UPDATE.bat` → GitHub Actions build chvíľu trvá. Ak Event odkaz
po nainštalovaní 1.9.5 stále nefunguje, daj vedieť presne čo sa stane pri kliku (nič, chybová hláška,
iná stránka...) - bez toho neviem ďalej diagnostikovať niečo, čo v kóde vyzerá správne.

**Čo som spravil:** Order kód v Tickets aj Inventory je teraz **vždy** odkaz na Order Detail, bez ohľadu
na `allowCrossLinks` - presne ten istý vzor, aký má Sale kód na Sales alebo Order kód na Orders (vlastný
záznam, nie krížová sekcia). Event zostáva pod `allowCrossLinks` prepínačom - to je podľa mňa naozaj
krížová sekcia (iná položka v bočnom menu), nie "vlastný záznam" tejto tabuľky. Keďže `allowCrossLinks`
je dnes `true` na oboch stránkach, ktoré túto tabuľku používajú, táto zmena **nezmení, čo dnes vidíš** -
len robí Order odkaz nezávislým od toho prepínača do budúcna, namiesto toho, aby sa naň len tak viezol.

**Toto je moja interpretácia tvojej vety o "vytvoriť aj v tickets tickets a v inventory inventory" - nie
som si 100% istý, že som pochopil presne, čo si tým myslel.** Pýtam sa na to nižšie (otázka v tejto správe) -
ak som netrafil, over prosím výber a napíš mi to inak, pokojne aj jednou vetou s príkladom.

---

## 4. Zmenené súbory

**Frontend (`src/`) - jediné zmeny tohto vydania:**
- `pages/Sales.tsx` - Platform filter zúžený na sale/both, opravený krížový komentár
- `pages/Settings.tsx` - Data sekcia: 3 karty pod sebou namiesto 2-stĺpcovej mriežky
- `pages/Tickets.tsx` (zdieľané s Inventory) - Order kód vždy odkaz, opravený krížový komentár, rozšírený
  historický komentár nad tabuľkou

**Verzia (6 súborov, ako vždy):** rovnaký postup ako minule, `Cargo.lock` opäť len vlastný `tiqr-manager`
balík (nesúvisiaci `indexmap` v `1.9.3` nedotknutý), `release.ps1` commit-message prepísaný na toto kolo,
`1-CLICK-UPDATE.bat` CRLF overené `file` príkazom po úprave.

**`src-tauri/` - nezmenené ani raz** (potvrdené `find -newermt`, sekcia 6).

---

## 5. Testy a build

Žiadne nové Rust testy - nezmenil sa ani jeden `.rs` súbor, existujúca sada (166 `#[test]`, 3 ignored) je
úplne nedotknutá. `cargo test`/`cargo check` by tu aj tak nemali čo nanovo overiť. TypeScript build sa
stále nedá reálne spustiť (`node_modules` prázdny, dlhodobé obmedzenie sandboxu) - namiesto toho znova
`ts.createSourceFile` nad všetkými 6 dotknutých/súvisiacich súborov (**0 syntaktických chýb**) + kontrola
párovania `{}`/`()` na všetkých upravených súboroch vrátane `release.ps1`. Skutočné overenie prebehne u
teba cez `1-CLICK-UPDATE.bat` → GitHub Actions.

---

## 6. Regresia a DO NOT TOUCH

`find -newermt` potvrdzuje presne tie 3 súbory zo sekcie 4 - nič v `src-tauri/`, nič vo `finance.rs`/
`money.rs`/migráciách/`refund_sale_impl`/`backup.rs`/`csv_import.rs` sa dnes ani len neotvorilo.

---

## 7. FOUND BUT NOT TOUCHED

- **Event odkaz na Tickets/Inventory** - podľa priameho prečítania kódu by mal fungovať rovnako ako Order;
  nenašiel som žiadny rozdiel v spracovaní. Nechané tak, keďže by som inak menil niečo, o čom neviem s
  istotou, že je to skutočne pokazené (pozri sekciu 3 vyššie).
- **Dashboard "New Event/Order/Sale/Import/Export" riadok** - druhýkrát spomenuté, stále bez konkrétneho
  miesta. Pýtam sa nižšie namiesto hádania, keďže minulý raz som už raz nechal čakať a teraz by ďalšie
  hádanie mohlo znamenať tretie kolo navyše, ak netrafím.

---

## STOP

Čakám na odpovede k dvom otázkam nižšie (Tickets/Inventory odkaz, Dashboard riadok) - ostatné tri veci sú
hotové a môžeš ich rovno skúsiť cez `1-CLICK-UPDATE.bat`.
