# TIQR Manager 1.9.10 — Pulls Date column, Dashboard fix, Event form reorder, Orders Notes, Order Detail row

Report k verzii **1.9.10**. Nadväzuje na 1.9.9 (Pull obor od 1.9.7, upravovaný v 1.9.8/1.9.9 - prečítal
som si všetky tri reporty aj aktuálny zdroj, nech mám presný kontext). Šesť vecí z tvojej správy, všetky
hotové. **Žiadna zmena v `src-tauri/` - opäť len frontend, vrátane celého Pulls backendu** (overené na
súborovom systéme, sekcia 7).

---

## 1. Pulls — Date ako vlastný stĺpec

Presne tá istá úprava, akú dostali Seats a More info v 1.9.9. Event stĺpec teraz nesie len meno eventu;
dátum má svoj vlastný nový stĺpec hneď za ním. Zoznam stĺpcov: Pull | For | Event | **Date** *(nové)* |
Seats | More info | Ks | Platform | Fee | Warning | Done.

## 2. Pulls — Seats a More info širšie

Seats: 92px → **200px** (dosť na celé "Sec 102 · Row 5 · Seat 12" aj na "General admission"). More info:
136px → **240px** (dosť na bežný mail aj kratšiu poznámku).

**Úprimne o dôsledku:** s týmito dvomi rozšíreniami a novým Date stĺpcom sa táto tabuľka už nezmestí do
pôvodnej 808px podlahy appky pri úplne najužšom podporovanom okne - potrebuje približne 1100px, inak sa
zobrazí vodorovný scrollbar (ktorý tam už existoval - `overflow-x-auto` - takže sa nič nerozbije, len sa
za istých okolností objaví posuvník, ktorý predtým nebol potrebný). Keďže si výslovne povedal, že chceš
vidieť všetko celé, toto je vedomý kompromis v prospech čitateľnosti, nie prehliadnutie.

---

## 3. Dashboard — period-switcher rectangle končí pri Custom

**Diagnóza:** ten "obdĺžnik inej farby" (biele/tmavé pozadie s okrajom okolo Today/1 Wk/.../Custom) je
samostatný `<div>` v bežnom toku stránky - na rozdiel od taburového prepínača hore (ten sedí vnútri
hlavičky, ktorá ho prirodzene orezáva), tento sa štandardne naťahoval na celú šírku obsahu, zatiaľ čo
tlačidlá vo vnútri zaberali len časť tejto šírky. Výsledok: farebný obdĺžnik pokračoval ďaleko za Custom
do prázdna.

**Oprava:** pridal `w-fit` - box teraz kopíruje presne šírku svojho obsahu, končí presne pri Custom.

---

## 4. New/Edit Event formulár — nové poradie polí

Presne podľa tvojho rozpisu:

Predtým (6 riadkov): Event name / [Category, Venue] / [Event date, City] / Country / Status / Notes
Teraz (5 riadkov): Event name / **[Category, Country]** / [Event date, City] / **[Status, Venue]** / Notes

Category teraz páruje s Country (predtým Venue), Status teraz páruje s Venue (predtým bol sám na celom
riadku, rovnako Country). Event date/City a Notes nezmenené. Žiadne pole sa nepridalo ani neodobralo, len
sa preusporiadalo - platí pre Edit aj New event (je to jeden zdieľaný formulár).

---

## 5. Event Detail — Potential Profit úplne dole

Presunuté z miesta hneď po Profit/Margin/ROI štatistikách (kde bolo pred Orders sekciou) na úplný koniec
stránky - teraz je pod Orders aj pod Tickets, tesne pred tlačidlami/modálmi. Obsah aj výpočet nezmenené,
len pozícia.

---

## 6. Orders — Notes stĺpec + Order Detail Platform/Notes/Currency v jednom riadku

**Zoznam Orders** dostal nový stĺpec Notes medzi Date a Platform: ORDER EVENT DATE **NOTES** PLATFORM QTY
SOLD TOTAL COST PAYMENT - presne v tomto poradí, ako si napísal. Skracuje sa s "..." pri dlhšom texte,
celý text po nabehnutí myšou (rovnaký vzor ako všade inde v appke).

**Rovnaký úprimný dôsledok ako pri Pulls:** s deviatimi stĺpcami namiesto ôsmich a Notes aj Platform
zaberajúcimi kus miesta, aj táto tabuľka teraz pri úplne najužšom okne potrebuje vodorovný scroll (rovnaký
`overflow-x-auto`, čo tam už bol). Pri bežnej šírke okna to nebudeš vidieť vôbec - týka sa to len
extrémneho minima.

**Order Detail** - karta s Platform/Currency (Notes bol predtým vlastný, len podmienene zobrazený riadok
pod nimi) je teraz jeden riadok v poradí **Platform, Notes, Currency**, presne podľa teba. Notes sa už
nezobrazuje len keď má obsah - teraz je to bežná bunka ako jej susedia, s "-" ak je prázdna (nech riadok
vždy vyzerá rovnako, nie raz 2 bunky, raz 3).

---

## 7. Zmenené súbory

**Frontend (`src/`) - jediné zmeny tohto vydania:**
- `pages/Pulls.tsx` - Date stĺpec, širšie Seats/More info
- `pages/Dashboard.tsx` - `w-fit` na period-switcher boxe
- `pages/Events.tsx` - preusporiadaný New/Edit Event formulár
- `pages/EventDetail.tsx` - Potential Profit presunutý na koniec
- `pages/Orders.tsx` - nový Notes stĺpec
- `pages/OrderDetail.tsx` - Platform/Notes/Currency v jednom riadku

**Verzia (6 súborov, ako vždy):** rovnaký postup, `Cargo.lock` opäť len vlastný `tiqr-manager` balík
(nesúvisiaci `indexmap` v `1.9.3` nedotknutý), `release.ps1` commit-message prepísaný na toto kolo,
`1-CLICK-UPDATE.bat` CRLF overené `file` príkazom po úprave.

**`src-tauri/` - nezmenené ani raz**, vrátane celého Pull backendu (`pulls.rs`, migrácie 005/006,
`models.rs`) - toto kolo sa ho vôbec netýkalo, potvrdené `find -newermt` (nič mimo zoznamu vyššie).

---

## 8. Testy, build, regresia

Žiadny `.rs` súbor sa nedotkol - existujúca sada (197 `#[test]` funkcií podľa 1.9.9, 3 ignored) je úplne
nedotknutá. TypeScript build sa stále nedá reálne spustiť (`node_modules` prázdny) - `ts.createSourceFile`
nad všetkými 6 upravených súborov ukázal **0 syntaktických chýb**, párovanie `{}`/`()` vyšlo vyvážené na
každom z nich. Pri `EventDetail.tsx` (presun celého bloku naprieč komponentom) som si po úprave ešte raz
overil, že sa text "Potential Profit" nezdvojil a že Orders/Tickets nadpisy ostali presne tam, kde boli.
`find -newermt` potvrdzuje presne tých 6 frontend súborov zo sekcie 7 - nič v `src-tauri/`, `finance.rs`,
`money.rs`, migráciách, `refund_sale_impl`, Backup/Restore ani Pull backende sa dnes ani len neotvorilo.

---

## 9. Čo NEBOLO zmenené

Celý Pull dátový model a backend (`pulls.rs`, `models.rs` časti pre Pull, migrácie 005/006), refund/resell
logika, `batch_id`/`SaleGroup`, `finance.rs`/`money.rs`, Backup/Restore, CSV import, migrácie 001-004,
Dashboard finančná logika (len jeden CSS box sa opravil), existujúce Sales/Tickets/Inventory
filtrovanie/hľadanie/radenie, Settings routing, `supplier_id`.

---

## STOP

Toto je všetkých šesť vecí z tvojej správy. Mimochodom, z 1.9.8/1.9.9 report-ov ešte visia dve otvorené
otázky o Pulls Warning (3 dni ako práh, a či má po evente ostať červené počítanie "overdue" alebo úplne
zmiznúť) - nespomenul si ich v tejto správe, tak predpokladám, že súčasné správanie vyhovuje, ale ak nie,
daj vedieť.
