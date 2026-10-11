# TIQR Manager 2.90.0 — report

Prvý implementačný blok z kompletného redesign mandátu. Píšem to rovno:
**mandát nie je hotový.** Čo je hotové, je hotové naozaj; čo nie je, je
vymenované nižšie menovite.

---

## 1. Čo audit našiel — a čo to zmenilo

Všetkých osem dokumentov, na ktoré sa mandát odvoláva, **existuje** a je
reálnych. Prečítal som ich a otvoril som aj preview artifact Events labu,
takže som nasadzoval to, čo si naozaj vybral, nie to, čo by som si domyslel.

A hneď prvý nález zmenil poradie práce:

> **2.89.0 — to, čo som ti poslal včera — porušuje THE REGISTER.**

Register hovorí doslova: *„vlasové linky, 8 px rádius, **žiadne tiene,
prechody** ani ilustrácie"*. Ja som na každú kartu nasadil tieň **aj**
gradient. Nebolo to rozhodnutie proti registru — ja som ten register v tej
chvíli nečítal.

Oboje je preč. Aj mäkké nadvihnutie, ktoré tam pridalo ešte 3.0.

---

## 2. THE REGISTER, nasadený globálne

| časť registra | predtým | teraz |
|---|---|---|
| Panel | 12 px rádius, tieň + gradient | **8 px, len vlasová linka** |
| Popisok | `font-medium` | **`font-semibold`**, ako register predpisuje |
| Field / Bar | neexistovalo | **`.field`, `.field-bar`** (16 × 28 px) |
| Čísla | proporcionálne | **mono** (`.num`) |
| Riadok čísel | päť kariet | **jeden panel, delený linkami** |

Mono je zo **systémového stacku** — žiadny nový súbor, appka ostáva offline.

---

## 3. Riadok čísel prestal byť riadkom kariet

**Toto je odpoveď na „vsetky okienka ostali rovnake".**

V registri **riadok hlavných čísel nie je päť kariet.** Je to jeden panel
a v ňom čísla oddelené vlasovými linkami. Každý lab, ktorý register nasadil,
to kreslil takto.

A nie je to kozmetika. Päť orámovaných krabíc dá piatim číslam **rovnakú
vizuálnu váhu ako tabuľke pod nimi** a čítajú sa ako päť samostatných
objektov, ktoré musíš prejsť jeden po druhom. Jeden panel so štyrmi predelmi
sa číta ako **jeden súhrn** — čím to aj je.

Takže si mal pravdu aj v tom, čo si povedal predtým: problém bola karta,
nie jej výplň.

**Prerobené:** Dashboard, EventDetail (3 riadky), Finance Overview (2 riadky).

`StatCard` som **zámerne nechal** exportovaný — `OrderDetail` a `SaleDetail`
ho balia inak. Prerobiť ich naslepo v tom istom kole je presne to, čím sa
redesign rozbije.

---

## 4. Events — hotové podľa usadeného dizajnu

`EVENTS-LAB-LOG.md` §07: **01 a 01**, usadené 30.09.

### Zoznam — *The Table*

Tvojich sedem stĺpcov ostáva nedotknutých (Stock padol v 2.76.0, ako si
pýtal). Pribudli tri veci:

- **Deň pod dátumom** — „in 2 days", „49 days ago", „cancelled". Register
  dovoľuje jeden akcent a ten patrí výberu, takže riziko **nesmie byť
  farba**. „in 2 days" vedľa riadku so štyrmi nepredanými lístkami je to
  varovanie, a číta sa bez palety.
- **Mono čísla** — stĺpec peňazí, ktorý je mono aj tabular, sa medzi riadkami
  nerozíde.
- **Počet riadkov, ktoré sa nedajú sčítať**, nad tabuľkou.

To posledné stojí za vetu navyše. Per-riadok „Mixed" fungovalo **vždy**
(`formatMoneyOrMixed` — a `REDESIGN-OVERVIEW.md` §3.1 si tú vlastnú chybu
opravuje). Čo chýbalo: **nič ti nepovedalo, že sa to deje.** Musel si si to
všimnúť v bunke. Chyba, ktorú si táto appka stále opakuje, sa neprejaví ako
zlé číslo — prejaví sa ako **neprítomnosť**, a tá nemá pixely.

### Detail — *One Page*

Záložky Overview a Sales sú **preč**. Event je jeden scroll. Lab to
zdôvodňuje jednou vetou, ktorú neviem povedať lepšie:

> *„Overview a Sales nikdy neboli dve rôzne otázky — boli to dve polovice
> jednej otázky."*

`onSwitchTab` bol mŕtvy prop: deklarovaný v typoch, v tele nikdy nezavolaný.

---

## 5. Dva konflikty — zapísal som ich, nehádal

**Pulls.** Mandát hovorí „DO NOT LEAVE IT BEHIND". Dokumentácia hovorí
opak — `PULLS-LAB-LOG.md` má sekciu *Rozhodnuté*: **„Pulls ostáva tak, ako
je. 30.09. — nič sa teraz nestavia."** Tvoj novší pokyn má prednosť, ale
nebudem predstierať, že to nie je rozpor. Register pass na Pulls je
nedorobok; funkcie z labu (Receipts, Fees Owed) som nestaval, lebo tie menia
finančné správanie a to je chránená oblasť.

**Syne a Instrument.** Vybral si ich a ja ich **nedodám**. Appka je
offline-first a bundluje len dva Bricolage subsety; tie font súbory nemám
a stiahnuť ich neviem. Mono polovicu registra som dodal zo systémového
stacku. Ak chceš Syne/Instrument naozaj, treba tie `.woff2` dostať do
`src/assets/fonts/` — mechanizmus na to už existuje.

---

## 6. Čo ešte NIE JE spravené

Aby to bolo jasné a nie schované v odseku:

| Oblasť | Stav |
|---|---|
| Inventory | nedotknuté |
| Sales | nedotknuté |
| Pulls | nedotknuté (zaparkované, viď vyššie) |
| Finance — Transactions, Accounts, Reports | nedotknuté |
| Notes — Two Pane, Pin to Record, Quick Note, Real Checklist, Templates | nedotknuté |
| Settings — Import Preview + podsekcie | nedotknuté |
| Bankový výpis cez CSV | nepostavené |
| Restriction Stamp | nepostavené |
| Bulk Change Preview | nepostavené |

Tie tri funkcie dole sú všetky **náhľad pred zápisom** a dotýkajú sa
chránených oblastí (migrácie, hromadné zápisy, peniaze). Postaviť ich
naslepo, bez kompilátora, v tom istom kole ako plošnú zmenu vzhľadu, by
nebolo rýchlejšie — bolo by to nebezpečné.

---

## 7. Overenie

**Pozrel som sa na to**, nielen prečítal: vyrenderoval som pred/po riadku
čísel a novú tabuľku Events s tvojimi vlastnými číslami. Rozdiel je
štrukturálny — päť krabíc verzus jeden delený panel — nie farebný.

Statické kontroly, každá najprv overená na známe pokazenom súbore:

| Kontrola | Výsledok |
|---|---|
| Vyváženie zátvoriek, celý `src/` | 65/65 OK |
| Vyváženie `index.css` | 108/108 OK |
| Komentárové chyby A + B | 0 výskytov |
| Každé `<Figure>` má import | 3/3 OK |

**Nie je to kompilácia.** Node ani cargo na tomto Macu nie sú — CI je prvý
kompilátor. A dev server sa z toho istého dôvodu spustiť nedá.

---

## 8. Verzia

2.89.0 → **2.90.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
