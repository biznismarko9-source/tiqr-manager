# TIQR Manager 2.64.0

Štyri veci, čo si vypísal. Jedna z nich bola vážnejšia, než vyzerala na fotke.

---

## 1. Tá fotka. Chýbali ti peniaze v číslach.

Písalo ti to **„Convert to EUR: € (2)"**. Previesť eurá na eurá nedáva zmysel —
ale to nebolo to zlé.

V kolónke s menou bol pri dvoch objednávkach **symbol `€`** namiesto kódu `EUR`.
Appka sa všade pýta *„je to EUR?"*, a `€` nie je `EUR`.

**Takže tie dve objednávky, ich lístky aj ich predaje vypadli zo všetkých súčtov
na Dashboarde.** Nie že by boli zle popísané. Neboli tam vôbec.

### Prečo sa to stalo

Toto sa už raz našlo. V `fx.rs` je funkcia `normalize_currency` a v jej vlastnom
komentári je presne tento problém popísaný — vtedy sa opravilo **prevádzanie**.

Ale nikto neopravil dve ďalšie veci: **riadky, čo už boli zapísané**, a **to, že
sa taký riadok dal zapísať znova**.

### Čo som spravil

**Opravil som uložené dáta** — migrácia naprieč **všetkými 14 tabuľkami**, čo
nesú menu. Naschvál všetky naraz: objednávka opravená na `EUR`, ktorej lístky by
ostali na `€`, by bola horší stav, než z akého sme začali.

**Opravil som vstup** — pri vytvorení aj úprave objednávky sa mena narovná ešte
predtým, než sa čokoľvek zapíše.

**Nevymýšľam si.** Prevádzajú sa len jednoznačné symboly a tá tabuľka je
odpísaná z `normalize_currency`, aby sa nemohli rozísť. **`kr` sa naschvál
nechá tak** — je švédska, nórska aj dánska, a vybrať jednu by znamenalo hádať
o tvojich peniazoch.

### Overené na špinavej databáze

| Čo bolo | Čo je |
|---|---|
| `€` | `EUR` |
| `eur` | `EUR` |
| ` GBP ` | `GBP` |
| `£` | `GBP` |
| `$` | `USD` |
| `kč` | `CZK` |
| `Ft` | `HUF` |
| `lei` | `RON` |
| `zł` | `PLN` |
| **`kr`** | **`kr` ostáva** |

Banner predtým ponúkal jedenásť „iných mien". **Teraz ponúka len tie, čo naozaj
iné sú.**

Ešte som overil, že **druhé spustenie už nezmení nič** a že **na čistej databáze
migrácia nezapíše ani jeden riadok**.

---

## 2. Listings preč z Events

V Evente ostali **dve zložky: Overview a Sales**.

Zmazal som `ListingsTab`, `ListingsBulkBar`, `TicketListingFormModal` a ich
konštanty — **1 135 riadkov**, plus 21 importov, ktoré tým ostali visieť.

### Čo som naschvál nechal

**Tabuľku `ticket_listings`, migráciu 022 a všetkých sedem príkazov.** A stĺpec
**„Listing price" v objednávke**, ktorý z tých dát stále číta.

Je to ten istý postup ako pri Sheets v 2.61.0: **pýtal si si preč sekciu, nie
záznamy.** Zmazať samotnú tabuľku je iné rozhodnutie, je nevratné a chcem naň
tvoje slovo.

---

## 3. a 4. Dátum eventu

**V objednávke** aj **v predaji**, medzi `Ticket` a `Seat`.

Backend meniť netreba — `eventDate` sa už posielal, len sa nikde nezobrazoval.

**V predaji sa číta pre každý riadok zvlášť**, nie z hlavičky predaja. Keď
predáš lístky z viacerých eventov naraz, hlavička žiadny dátum nemá, ale každý
riadok svoj má.

### Čo to stálo

Tie tabuľky majú **premerané šírky stĺpcov, ktoré musia dať 100**. Nový stĺpec
som teda nepridal, ale **zaplatil**:

- **objednávka** — desať bodov rovno zo `Seat`, ktorý mal 49 % a 53 %
- **predaj** — `Seat` má v úzkej tabuľke len 10 % a sám to nezaplatí, tak sa to
  vzalo z `Order`, `Profit`, `Delivery`, `Payout` a stĺpca akcií

Obe tabuľky, obe šírky, stále presne 100.

---

## Čo som overil

**Spustené, nie prečítané:**

- **migrácia na špinavej databáze** — všetkých 10 zápisov mien sedí, `kr` ostal,
  druhý beh nič nezmení, čistá databáza ostane nedotknutá
- **39 migrácií** na čistej databáze
- **103 SQL príkazov** v `orders.rs` a `dashboard.rs` proti skutočnej schéme
- **241 príkazov** Rust ↔ appka 1:1
- **7 príkazov pre listings** je stále registrovaných, tabuľka je na mieste
- **šírky stĺpcov** — obe tabuľky, obe varianty, súčet presne 100
- **žiadny nepoužitý import** v troch upravených stránkach
- **zátvorky** vo všetkom, čoho som sa dotkol

### Jedna vec, ktorú nezamlčím

`OrderDetail.tsx` hlási mojej kontrole nerovnováhu zátvoriek. **Bola tam aj
v 2.63.0** — porovnal som to priamo proti predošlému balíku, číslo je rovnaké.
Je to apostrof v texte, na ktorom sa moja kontrola pomýli, nie chyba v kóde.

**Neoverené:** appku nezostavím, nemám tu Node ani Rust. To spraví tvoj build.

---

## Po inštalácii

Migrácia zbehne sama pri prvom spustení. **Pozri sa hneď na Dashboard** — ten
žltý banner by mal zmiznúť a súčty by mali byť vyššie o tie dve objednávky,
čo tam doteraz chýbali.

Ak by banner ostal, znamená to, že máš naozaj ešte niečo v inej mene — a vtedy
bude v tom tlačidle napísaný **kód**, nie symbol.
