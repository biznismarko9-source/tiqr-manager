# TIQR Manager 1.9.8 — Pulls: úprava podľa spätnej väzby

Report k verzii **1.9.8**. Priama reakcia na tvoju spätnú väzbu k 1.9.7 - tri konkrétne veci, ktoré si
napísal po tom, čo si Pulls videl naživo.

---

## 1. Event stĺpec teraz nesie aj Seats a More info

**Čo si napísal:** "tu napiseme seats a taktiez pridame to more info, lebo 90% casu sa tu dava mail, na
ktorom su listky".

V stĺpci Event je teraz pod menom eventu a dátumom aj miesto (sekcia/rad/sedadlo, ak je vyplnené) a pod tým
More info - presne preto, že tam najčastejšie píšeš mail, na ktorý majú prísť lístky, takže to má byť vidieť
rovno v zozname, nie len po otvorení formulára. More info je skrátené (s "..." pri dlhšom texte), celý text
uvidíš po nabehnutí myšou.

---

## 2. Seats → Section / Row / Seat (ako pri Tickets)

**Čo si napísal:** "pri listovani zmenime ale seats a nahradime ho ako pri order atd so sector, row, seats".

Pôvodné jedno voľné textové pole "Seats" som nahradil tromi poliami - **Section / Row / Seat** - presne taký
istý tvar, aký majú aj Tickets (`section`/`rowLabel`/`seat`). Vo formulári sú teraz tri samostatné políčka;
pri všeobecnom vstupe (general admission) necháš všetky tri prázdne, presne ako pri lístkoch dnes.

**Databáza:** pridal som novú migráciu (`006_pulls_seat_fields.sql`), ktorá pridáva tri nové stĺpce
(`section`, `row_label`, `seat`) do tabuľky `pulls`. **Tvoj existujúci pull (Bad Bunny Puerto Rico) som
neprišiel o dáta** - čo si mal predtým napísané v starom poli "Seats" ("General Admision"), appka pri
prvom spustení tejto verzie automaticky skopíruje do nového poľa "Seat", takže to hneď uvidíš pri otvorení
na úpravu (môžeš si to tam poprípade rozdeliť do Section/Row/Seat, ak by si chcel byť presnejší). Staré pole
`seats` samotné som v databáze nechal (len sa už nepoužíva) - vysvetlenie prečo je v sekcii 5.

---

## 3. Transfer deadline preč, namiesto toho automatické upozornenie

**Čo si napísal:** "transfer deadline mozes odstranit a skor nastavit nejaky warning o tom 3 dni pred
eventom kazdy den davam warning aby sa nezabudlo na transfer, ked bude done tak zmizne".

Ručné pole "Transfer deadline" som z formulára aj zo zoznamu úplne odstránil. Namiesto neho appka teraz sama
počíta upozornenie z **dátumu eventu**:

- **3 dni pred eventom** (a menej) sa v stĺpci "Warning" objaví žltý štítok, napr. "2d left".
- **V deň eventu** sa zmení na červený "Today!".
- **Po evente**, ak transfer stále nie je hotový, ostáva červený a ukazuje, koľko dní je to už po termíne
  ("3d overdue") - teda presne to "každý deň dávam warning, aby som nezabudol", len sa to sfarbí červeno,
  aby to bolo naliehavejšie.
- **Akonáhle zaklikneš "Transfer done"** (v zozname alebo vo formulári), upozornenie okamžite zmizne.

Toto číslo 3 dni je zatiaľ pevne dané v kóde (nie nastaviteľné v appke) - ak by si chcel iný počet dní,
napíš, je to jednoriadková zmena.

Vo formulári teraz namiesto poľa na dátum je krátka poznámka, ktorá to isté vysvetľuje, aby nebolo
prekvapenie, prečo tam dátum na zadanie chýba.

---

## 4. Ako to teraz vyzerá - stĺpce zoznamu

Pull | For | Event *(meno · dátum · miesto · more info)* | Ks | Platform | Fee | **Warning** *(nové)* | Done

Stĺpec "Deadline" je preč, nahradil ho "Warning" opísaný v sekcii 3.

---

## 5. Prečo staré `seats` pole ostalo v databáze (len nepoužívané)

SQLite vie stĺpec aj odstrániť (`DROP COLUMN`), ale túto appka doteraz nikde nepoužila a nechcel som prvýkrát
skúšať práve na tvojich reálnych dátach bez možnosti si to sám otestovať kompilátorom (v tomto prostredí ho
nemám - viď sekciu 7). Pridanie 3 nových stĺpcov + skopírovanie starých dát je bezpečnejšia cesta - staré
pole `seats` tam ostáva ticho ležať, appka ho už nikde nečíta ani nezapisuje, takže ťa nijako neobmedzuje.
Rovnaký prístup som použil aj pri `transfer_deadline` (stĺpec ostáva v databáze, len appka ho už nenastavuje).

---

## 6. Zmenené súbory

**Nové:**
- `src-tauri/migrations/006_pulls_seat_fields.sql`

**Upravené (backend):**
- `src-tauri/src/commands/pulls.rs` - prepísané SQL pre vytváranie/úpravu (nové stĺpce, prečíslované `?N`
  parametre - viď sekciu 7, ako som to overoval), rozšírené hľadanie o Section/Row/Seat, 2 nové testy
  (celkovo 26)
- `src-tauri/src/models.rs` - `Pull`/`PullInput`/`PullEditInput` - `seats` nahradené `section`/`rowLabel`(
  interná `row_label`)/`seat`, `transferDeadline` odstránené zo vstupov (ostáva len na čítanie)
- `src-tauri/src/db.rs` - zaregistrovaná nová migrácia

**Upravené (frontend):**
- `src/pages/Pulls.tsx` - prerobený zoznam (Event stĺpec bohatší, nový Warning stĺpec) aj formulár
  (Section/Row/Seat namiesto Seats, Transfer deadline preč)
- `src/lib/types.ts` - rovnaké zmeny ako v `models.rs`, len na strane TypeScriptu

**Verzia (6 súborov, ako vždy):** 1.9.7 → 1.9.8 vo všetkých 6 miestach, `release.ps1` commit-message
prepísaná na toto kolo, `1-CLICK-UPDATE.bat` CRLF overené binárne po úprave.

---

## 7. Testy, build, regresia - úprimne, s extra opatrnosťou tento krát

Prepisovanie SQL pri pridávaní stĺpcov znamená ručné prečíslovanie `?1, ?2, ...` parametrov v dvoch
príkazoch (vytváranie aj úprava pullu) - presne ten typ chyby, ktorá by **nespôsobila chybu pri kompilácii**,
len by ticho zapísala nesprávnu hodnotu do nesprávneho stĺpca. Keďže v tomto prostredí stále nemám funkčný
`cargo` ani `npm install` (rovnaký sieťový blok ako v predošlých kolách), venoval som tomuto kroku extra
pozornosť:

- Každý parameter v oboch SQL príkazoch som ručne prešiel trikrát - raz pri písaní, raz pri vlastnej kontrole
  po napísaní, raz cez nezávislého review agenta, ktorého som výslovne poprosil vypísať si celé priradenie
  stĺpec → `?N` → parameter pre každé číslo zvlášť (nielen povedať "vyzerá to dobre").
- Druhý nezávislý review agent prešiel frontend rovnako dôkladne - importy, typy, a špeciálne miesta, kde by
  TypeScript mohol mať problém s `null` hodnotami (dátum eventu, ktorý môže chýbať).
- Pri mojom vlastnom prechode kódu pred odoslaním na review som ešte našiel a opravil chýbajúci import
  (`centsToDecimalString`), ktorý by inak appku reálne rozbil pri otváraní formulára na úpravu - presne
  preto robím tento krok vždy, nie len "napísať a odoslať".
- TypeScript syntax check (rovnaký ako v 1.9.7) - **0 chýb** na oboch dotknutých frontend súboroch.
- Kontrola vyváženosti zátvoriek vo všetkých upravených `.rs` súboroch - v poriadku.
- Nové testy (2 pridané, 26 spolu v `pulls.rs`) sú napísané a mali by prejsť, ale `cargo test` tu reálne
  spustiť neviem - rovnaké obmedzenie ako minule.

**Regresia:** migrácia 006 je čisto pridávacia (`ALTER TABLE ... ADD COLUMN`), nemení ani nemaže žiadny
existujúci stĺpec ani tabuľku. Nič iné v appke (Events/Orders/Tickets/Sales, `finance.rs`, `money.rs`,
Backup/Restore, CSV import, migrácie 001-004, Dashboard) som sa dnes nedotkol.

---

## 8. Čo NEBOLO zmenené

Refund/resell logika, `batch_id`/`SaleGroup`, zoskupovanie Tickets/Orders/Event, `finance.rs`/`money.rs`,
Backup/Restore, CSV import, migrácie 001-004, Dashboard finančná logika ani layout, existujúce Sales
filtrovanie/hľadanie/radenie, Settings routing, `supplier_id`. Financie z Pullov (tvoja odmena) sú stále
úplne mimo Dashboardu, presne ako si chcel v 1.9.7.

---

## STOP

Toto sú tie tri veci z tvojej spätnej väzby. Konkrétne sa chcem opýtať na dve rozhodnutia, ktoré som urobil
sám bez pýtania (dávalo mi to zmysel, ale sú to reálne voľby):

1. **3 dni** ako práh na upozornenie - vyhovuje, alebo chceš iný počet dní?
2. Po prejdení dátumu eventu upozornenie **neostáva ticho**, ale zosilnie na červenú a počíta, koľko dní je
   to už po termíne - je to tak, ako si chcel ("každý deň warning"), alebo si predstavoval niečo iné (napr.
   aby po evente už neupozorňovalo vôbec, keďže lístky už boli použité)?
