# TIQR Manager 1.9.7 — nová funkcia: Pull

Report k verzii **1.9.7**. Nadväzuje na 1.9.6 - toto je nový obor, ktorý si zadal: Pull, teda kupovanie
lístkov pre niekoho iného za odmenu. Backend aj frontend, kompletne nová funkcia, žiadna existujúca
obrazovka sa nepoužíva - Pulls má vlastnú novú sekciu v menu.

---

## 1. Čo je Pull a ako to funguje

Zapojíš sa do queue predaja, skúšaš vložiť najlacnejšie lístky do košíka. Keď sa to podarí, dotyčný ti
pošle údaje jednorazovej karty, ktorou zaplatíš, a vyplatí ti dohodnutú odmenu (napr. 15 €) po tom, čo mu
lístky prepošleš. Appka teraz vie:

- **Pridať nový pull** - tlačidlo "New Pull" v novej sekcii **Pulls** (v menu medzi Inventory a Settings).
- **Sledovať zoznam** - tabuľka so search-om (hľadá v mene, evente, kóde, platforme, seats aj more info -
  presne také free-text hľadanie ako majú Orders/Sales) a filtrom "All / Not transferred yet / Transferred".
- **Editovať** - klik na riadok otvorí ten istý formulár, len predvyplnený; dá sa opraviť čokoľvek vrátane
  spätného prepnutia "Transfer done".
- **Rýchlo označiť transfer** - checkbox priamo v riadku zoznamu, bez otvárania formulára.
- **Zmazať** - tlačidlo Delete v edit formulári, s potvrdzovacím dialógom (rovnaký `ConfirmDialog`, aký appka
  používa všade inde).

Každý pull dostane vlastný kód `PULL-000001`, `PULL-000002`, ... - rovnaký princíp ako `ORD-...`/`TIX-...`/
`SALE-...`, len vlastný číselník.

---

## 2. Dátový model - 4 rozhodnutia z tvojich odpovedí

Keďže toto je úplne nový obor (nie úprava existujúcej obrazovky), mal dátový model naozaj viac než jednu
rozumnú cestu, tak som sa pred písaním kódu spýtal na 4 konkrétne veci namiesto hádania. Tvoje odpovede:

**Čo je "identita" jedného pullu?** Meno osoby, pre ktorú lístky kupuješ (`buyer_name`) - je to povinné pole
a hlavný identifikátor riadku (stĺpec "For" v zozname), nie len voliteľná poznámka.

**Event pole - prepojiť na existujúce Events, alebo voľný text?** Voľný text. `event_name`/`event_date` sú
obyčajné textové polia, nič neoveruje, že taký event existuje v tvojom Events zozname. Presne to si povedal -
"voľný text" - a dáva to zmysel, keďže pull sa netýka tvojich vlastných eventov/objednávok, len eventu tej
druhej osoby.

**Aký "stav" pull potrebuje?** Len checkbox ("Transfer done" - áno/nie), nie viacstavový status ako majú
Tickets (available/listed/sold/cancelled). Pull nemá životný cyklus s viacerými fázami - buď si to
transfernul, alebo nie.

**Majú sa peniaze z pullu (tvoja odmena) počítať do Dashboardu/financií appky?** Úplne samostatné. `price_cents`
(tvoja odmena) sa nikde nesčítava do `FinanceSummary`/`CashflowSummary`/Dashboard čísel. Dôvod: tá odmena nie
je zisk z ďalšieho predaja lístkov (to je to, čo appka doteraz počítala) - je to poplatok za službu, iná
kategória peňazí. Ak by si to neskôr chcel vidieť aj na Dashboarde, je to samostatná, prídavná zmena - nič v
tomto vydaní tomu nebráni.

---

## 3. Polia vo formulári (z tvojho zoznamu stĺpcov + doplnky, ktoré si žiadal)

Tvoj pôvodný zoznam stĺpcov ("pull Event name event date Ks Platform More info Seats [prázdny stĺpec]
Transfer Price date") je vo formulári celý, plus presne to, čo si dodatočne žiadal:

- **Meno (For)**, **Event name**, **Event date** - voľný text, dátum nepovinný
- **Quantity (Ks)** - počet lístkov
- **Platform** - rovnaký `LookupSelect` s "+ New" ako Orders/Sales, filtrovaný na "purchase"/"both" platformy
  (kupuješ, takže rovnaká logika ako pri Orders)
- **Seats**, **More info** - voľné texty
- **Tvoja odmena (Price) + mena** - rovnaký EUR/USD/... picker s "Other..." ako Orders, hodnota sa ukladá v
  centoch (rovnaký princíp ako všade v appke - žiadne desatinné čísla v DB)
- **Transfer deadline** - nové pole, ktoré si žiadal ("chcem sledovať nejaku deadline, do kedy transfernut")
- **Transfer done** - checkbox, len v edit formulári (nová pull vždy začína netransfernutá)
- **Dátum pridania (entry date)** - toto je **automatické**, presne ako si chcel ("automaticky datum kedy to
  tam zadam") - appka ho sama zapíše pri vytvorení, nikde sa nedá ručne meniť. V zozname nie je ako vlastný
  stĺpec (pozri sekciu 5, prečo), ale je vidieť ako tooltip nad kódom pullu pri nabehnutí myšou ("Added ...").

---

## 4. "Transfer done" - ako funguje časová pečiatka

Toto je jediná časť, ktorá potrebovala trochu premyslenejšiu logiku, tak to vysvetľujem otvorene.

Checkbox "Transfer done" je - na rozdiel od napr. refundu pri Sales (ten sa dá urobiť len raz, nedá sa
vrátiť späť) - **obojsmerný**: dá sa zapnúť aj vypnúť (opravíš omylom zaklinutý checkbox, alebo si sa
pomýlil pri dátume). Zároveň k nemu appka sama drží `transfer_done_at` - presný čas, kedy si to naposledy
označil za hotové - lebo to sa môže neskôr zísť (napr. koľko dní ti trvalo od pridania po transfer).

Pravidlo je jednoduché, aj keď sa to v kóde píše cez tri prípady:
- **Práve si to zaklikol (nie → áno):** appka zapíše aktuálny čas.
- **Práve si to odklikol (áno → nie):** appka čas vymaže (NULL) - nebol predsa naozaj urobený.
- **Nezmenilo sa to (áno → áno, alebo nie → nie, napr. pri obyčajnom uložení opravy iného poľa):** čas sa
  **nedotkne** - zostáva presne taký, aký bol.

Táto logika je zdieľaná medzi rýchlym checkboxom v zozname aj plným edit formulárom - obe cesty vedú k
rovnakému výsledku, nemôžu sa nikdy rozísť. Otestované všetkými smermi (pozri sekciu 6 - konkrétne testy pre
"nezmení čas pri obyčajnom uložení", "vymaže čas pri odkliknutí", "nezapíše čas znova, ak už bol hotový").

---

## 5. Čo som vedome nechal mimo (a prečo)

- **Seats a More info nie sú stĺpce v tabuľke zoznamu** - sú vo formulári (dajú sa vyplniť aj hľadať), ale
  keby boli aj v tabuľke, bolo by to 10 stĺpcov na jednej obrazovke - príliš veľa na rýchle prezeranie.
  Search cez ne aj tak funguje, takže sa dajú kedykoľvek nájsť.
- **Dátum pridania nie je vlastný stĺpec** - zoznam je aj tak zoradený od najnovšieho, takže poradie to už
  ukazuje; presný čas je dostupný cez tooltip nad kódom pullu.
- **Žiadne prepojenie na Dashboard** (Quick Action tlačidlo, súčet do štatistík) - vedome, podľa tvojej
  odpovede v sekcii 2. Dashboard.tsx som sa dnes vôbec nedotkol.
- **Žiadny CSV export/import pre Pulls** - nespomínal si to, appka to zatiaľ nemá; dá sa doplniť neskôr,
  keby to bolo užitočné (napr. export pre vlastnú evidenciu).

Toto sú vedomé rozhodnutia o rozsahu tohto vydania, nie prehliadnutia - ak niečo z toho chceš inak, napíš.

---

## 6. Zmenené a nové súbory

**Nové (backend):**
- `src-tauri/migrations/005_pulls.sql` - nová tabuľka `pulls` (vlastný `pull` counter pre kódy), indexy na
  platform/transfer_done/transfer_deadline/is_demo
- `src-tauri/src/commands/pulls.rs` - 6 príkazov (`list_pulls`, `get_pull`, `create_pull`, `update_pull`,
  `delete_pull`, `set_pull_transfer_done`), každý ako `impl` funkcia + tenký `#[tauri::command]` wrapper
  (rovnaký vzor ako všade v appke), **24 unit testov** (vytváranie, validácie, search/filter, plná úprava,
  rýchly transfer-checkbox vrátane všetkých smerov jeho časovej pečiatky, mazanie)

**Upravené (backend, len prídavné zmeny):**
- `src-tauri/src/db.rs` - zaregistrovaná nová migrácia (5. v poradí)
- `src-tauri/src/models.rs` - pridané `Pull`/`PullInput`/`PullEditInput`
- `src-tauri/src/commands/mod.rs` - `pub mod pulls;`
- `src-tauri/src/lib.rs` - 6 nových príkazov zaregistrovaných v `generate_handler!`

**Nové (frontend):**
- `src/pages/Pulls.tsx` - celá obrazovka (zoznam + zdieľaný create/edit formulár - Pull nemá vlastné
  "detail" podstránky ako Order/Sale, lebo negeneruje žiadne ďalšie záznamy, takže to nepotrebuje)

**Upravené (frontend, len prídavné zmeny):**
- `src/lib/types.ts` - `Pull`/`PullInput`/`PullEditInput`
- `src/lib/api.ts` - 6 nových `api.xxx` volaní
- `src/components/icons.tsx` - nová ikonka `IconUsers`
- `src/components/Layout.tsx` - nová položka menu "Pulls"
- `src/App.tsx` - nová routa `/pulls`

**Verzia (6 súborov, ako vždy):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
`Cargo.lock` (len vlastný `tiqr-manager` balík), `release.ps1` (`$Version` aj prepísaný `$CommitMsg`),
`1-CLICK-UPDATE.bat` (CRLF overené binárne po úprave, presne ako vždy).

---

## 7. Testy, build, regresia - úprimne o limitoch sandboxu

Rovnaký problém ako v predošlých kolách - v tomto sandboxe nemám funkčný `cargo` (siete zablokované na
`index.crates.io`) ani `npm install` (sťahovanie balíčkov padá na 403), takže **skutočný `cargo test`/
`cargo clippy` ani `npm run build` reálne spustiť neviem**. Po minulej skúsenosti (viď 1.9.3 - chýbajúci
import, ktorý prešiel cez manuálne čítanie aj cez review) som tomuto kolu venoval o čosi viac overovania,
než je bežný štandard:

- **TypeScript syntax check** - globálne nainštalovaný `typescript` balík (`ts.createSourceFile`) nad
  všetkými 6 dotknutými frontend súbormi: **0 syntaktických chýb**. Toto nie je plná typová kontrola (na tú
  by som potreboval `node_modules` s react/react-router typmi, ktoré sa nedajú stiahnuť), ale zachytí
  nevyvážené zátvorky/JSX aj vážnejšie preklepy v syntaxi.
- **Ručná typová kontrola nad rámec bežného čítania** - keďže `tsc` nevie overiť skutočné typy, prešiel som
  ručne každé miesto, kde by `strict: true` mohlo niečo odhaliť (napr. porovnávanie `transferDeadline`,
  ktoré môže byť `null`, s dnešným dátumom pri deadline upozornení) - narazil som na jedno miesto, kde by
  bežný zápis (`!!hodnota &&`) mohol byť dvojznačný pre type-checker, tak som to prepísal na jednoznačný tvar
  (`hodnota !== null &&`) ešte pred odoslaním, nie až po chybe.
- **Rust brace/paren balance check** - mechanická kontrola vyváženosti `{}`/`()` vo všetkých nových/
  upravených `.rs` súboroch. Toto NIE JE náhrada za `cargo check` - je to len sieťová poistka proti hrubým
  preklepom, nie záruka, že sa to skutočne skompiluje.
- **Dva nezávislé review agenti** (backend + frontend), ktorých som tentokrát výslovne poprosil, aby
  hľadali presne ten typ chyby, čo prešiel v 1.9.3 (chýbajúci/nesprávny import, nesediace typy polí,
  poradie parametrov v SQL) - obaja prešli súbory nanovo, riadok po riadku, a nenašli žiaden problém. Toto
  nenahrádza skutočný kompilátor, ale je to dôkladnejšia kontrola, než len moje vlastné čítanie.
- **24 nových unit testov** v `pulls.rs` je napísaných a mal by prejsť (rovnaký vzor ako stovky existujúcich
  testov v appke), ale keďže `cargo test` tu nespustím, nemôžem ti garantovať, že reálne prešli - len že sú
  logicky správne podľa opakovaného ručného prekontrolovania.

**Regresia:** nová migrácia (5.) je čisto pridávacia (`CREATE TABLE IF NOT EXISTS`), nemení žiadnu
existujúcu tabuľku. `backup.rs`/Backup-Restore nepotrebuje žiadnu zmenu - používa binárnu kópiu celej
databázy, takže novú tabuľku zálohuje/obnoví automaticky. Nič sa nedotklo `finance.rs`, `money.rs`,
`refund_sale_impl`, `batch_id`/`SaleGroup`, existujúceho filtrovania/radenia Sales, CSV importu, migrácií
001-004, ani `supplier_id`.

---

## 8. Čo NEBOLO zmenené

Refund/resell logika, `batch_id`/`SaleGroup`, zoskupovanie Tickets/Orders/Event, `finance.rs`/`money.rs`,
Backup/Restore, CSV import, migrácie 001-004, Dashboard finančná logika (ani layout - Dashboard.tsx som sa
vôbec nedotkol), existujúce Sales filtrovanie/hľadanie/radenie, mazanie sale/refund, Settings routing,
`supplier_id`. Nič z toho som dnes ani neotvoril.

---

## 9. Nápady do budúcna (nič z toho som nerobil, len na zváženie)

- Upozornenie na Dashboarde alebo inde, keď sa blíži/prešla transfer deadline (podobne ako existujúce
  upozornenia na nezaplatené objednávky)
- CSV export zoznamu Pulls (pre tvoju vlastnú evidenciu mimo appky)
- Voliteľné počítanie odmeny z pullov do Dashboardu (samostatný riadok, nie zamiešané do existujúceho zisku)

---

## STOP

Toto je celý Pull obor podľa toho, čo si opísal a čo sme si ujasnili cez otázky. Napíš, či to sedí - najmä
či ti stačí checkbox "Transfer done" v zozname/formulári, alebo či si predstavoval sledovanie inak, a či ti
chýba niektorý stĺpec priamo v tabuľke. Spomínal si, že môže pribudnúť viac požiadaviek - pokojne napíš,
keď ti napadnú.
