# TIQR — prestavba, celkový prehľad

Stav k 30.09.2026. Toto je jediný dokument, ktorý drží všetko
pokope. Jednotlivé denníky sú podrobnejšie, tento je na to, aby si
nemusel otvárať sedem súborov.

---

## 1. VZHĽAD JE UZAVRETÝ

Vybral si Look panel z finance labu a povedal: takto, všade. Zapísal
som ho ako skutočný design system — **THE REGISTER**, dvanásť častí:

- popisok je malý, utlmený, polotučný a **vždy NAD** svojím ovládačom
- segmented control je **zapustená dráha s vystúpenou vybranou pilulkou**
- medzery medzi skupinami 16 × 28 px
- **jeden akcent, len na výber** — nič dekoratívne
- vlasové linky, 8 px rádius, žiadne tiene, prechody ani ilustrácie
- písmo humanistický bezpätkový, čísla mono

Odvtedy je ten súbor prekopírovaný do **piatich oblastí bez zmeny
jedinej hodnoty**. To je celý zmysel toho, že sa zapísal: druhá oblasť
už nemá čo pri vzhľade rozhodovať. Vďaka tomu trvá lab popoludnie a nie
týždeň.

**Dôsledok, ktorý sa ukázal ako dobré obmedzenie:** keď je jediný akcent
vyhradený pre výber, **riziko nesmie byť farba**. Nesie ho teda poradie,
sekcia a pozícia. Žiadny červený riadok, žiadna oranžová bodka. Zoznam
zoradený podľa toho, čo horí, povie viac než zoznam s farebnými bodkami.

---

## 2. ČO JE ROZHODNUTÉ

| Oblasť | Vybrané |
|---|---|
| **Dashboard** | Noir · TIQR v Syne, zvyšok Instrument · normálna hustota · Slight rohy · návrh **01** na všetkých troch záložkách · **bankový výpis cez CSV** |
| **Settings** | **Import Preview** dnu, zvyšok ostáva — úloha je spraviť ju profesionálnejšou, nie prestavať |
| **Notes** | **Two Pane** + **Pin to Record**, **Quick Note**, **Real Checklist**, **Note Templates** — zapojené priamo do návrhu, nie nalepené vedľa |
| **Events** | **01 a 01** — *The Table* na zozname, *One Page* vnútri |
| **Pulls** | zaparkované, lab slúži ako zoznam na neskôr |
| **Finance** | otvorené |
| **Sales / Inventory** | **Restriction Stamp** + **Bulk Change Preview** · návrhy otvorené |

---

## 3. ŠTYRI VECI, KTORÉ NIE SÚ DIZAJN

Toto sú nálezy zo skutočného kódu. Nie návrhy — veci, ktoré platia teraz.

### 3.1 OPRAVA: zmiešaná mena je už vyriešená

**Toto som tvrdil zle a opakovane.** Písal som, že príznak `currency`
existuje a žiadna obrazovka ho nečíta. **Nie je to pravda.**

`Events.tsx` prepúšťa Cost, Revenue aj Profit cez `formatMoneyOrMixed`,
ktorý pri `currency === null` vypíše **„Mixed"** namiesto čísla.
`EventDetail.tsx` má na to celú vetvu s jantárovým riadkom: *„This event
has tickets in more than one currency, so these numbers can't be combined
into one here."* Aj Recap to rieši.

Takže je to **hotové, a hotové dobre**. Žiadne zlé číslo na obrazovke nie je.

Odkiaľ tá chyba vznikla: prečítal som komentár pri type a domyslel si, že
to nič nečíta — bez toho, aby som si našiel konzumentov. Potom som tú istú
vetu napísal do zadania agentovi, agent mi ju vrátil ako zistenie, a ja
som si ju odobril vlastným tvrdením. To je presne ten kruh, ktorému sa
mám vyhýbať.

### 3.2 Reálne chyby v Settings — OPRAVENÉ v 2.66.0

- **Upozornenia sa pri prvom zapnutí neuložia.** Klikneš checkbox,
  formulár sa zbalí, odznak ukáže „Enabled", Save zmizne, neuložilo sa nič.
- **Obnova staršej kópie z Drive nemá žiadne potvrdenie** — prepíše celú
  databázu a reštartuje. To isté „Take theirs" a „Sync down".
- **„Combine both" klame.** Píše „nothing is deleted", ale zlúčenie maže
  riadky, ktoré druhý počítač zmazal.
- Keď sa načítanie stavu nepodarí, **vyzerá to ako platný stav**.

### 3.3 Poplatky z Pulls nevidí Finance

V kóde sú označené ako *informational only*. Reálne peniaze, ktoré si
zarobil aj zaplatil, nevidí žiadny súčet v apke. Úprimná odpoveď na „majú
tam ísť?" je **áno pre prijaté, nikdy pre dohodnuté** — preto to musí
stáť na *Receipts*, nie na tej fajke.

### 3.4 Restrictions sú na lístku, a to nie je náhoda

Migrácia 037 to hovorí vlastnými slovami: *one order can easily be two
clear-view seats and two behind a pillar.* Pýtal si si výber pri novej
objednávke — odpoveď nie je presunúť ich na objednávku, ale **nastaviť
pri vytváraní predvoľbu, ktorú prevezme každý lístok a od ktorej sa
ktorýkoľvek môže líšiť**. Picker je formulárový prvok, nie pole:
neukladá sa nič. Druhá kópia dát na objednávke, ktorú by čítal nejaký
súčet, je **štrukturálne ten istý tvar ako incident s €**.

---

## 4. JEDNA VEC, KTORÁ SA VRACIA VŠADE

Incident s `€` namiesto `EUR` nie je uzavretá príhoda. Je to **tvar
chyby**, ktorý sa objavil v každej jednej oblasti:

- Dashboard: riadky vypadnuté zo súčtu
- Events: zmiešaná mena sčítaná ako jedna
- Pulls: meny zlúčené do jedného čísla
- Inventory: objednávka bez kódu meny
- Notes: nič — a to je v poriadku

Nezobrazil sa ako **zlé číslo**. Zobrazil sa ako **neprítomnosť**, a tá
nemá pixely. Preto sa v labe opakuje jedno riešenie: **každý súčet
povie, z koľkých riadkov je**, a pod ním riadok „X sa nepočítalo".
Červený segment má pevnú minimálnu šírku, aby sa jeden vypadnutý riadok
nikdy nevykreslil ako nič.

---

## 5. PRAVIDLÁ, KTORÉ DRŽALI CELÝ ČAS

- Nič nebeží na pozadí, nič nebeží kým je apka zavretá
- Nič si nedomýšľa číslo, ktoré si nezadal — kde údaj chýba, tam chýba
- Nič nepíše do tvojho živého Google sheetu
- Žiadne trhové dáta, žiadne porovnania s inými, žiadne rady
- Peniaze ostávajú celé centy, súčty nikdy nekrížia meny
- Nič z odmietnutých vecí sa nevrátilo, ani prezlečené
- Každý náhľad funkcie ukazuje **chytenú chybu**, nie zelenú fajku —
  náhľad, čo sa vráti celý zelený, ťa naučí, že funkcia nič nerobí

---

## 6. LABY

| Oblasť | Odkaz | Obsah |
|---|---|---|
| Dashboard | [log](DASHBOARD-LAB-LOG.md) | 30 návrhov, 19 funkcií |
| Settings | [log](SETTINGS-LAB-LOG.md) | 10 návrhov, 12 funkcií, 9 reálnych chýb |
| Finance | [log](FINANCE-LAB-LOG.md) | 60 návrhov, 12 funkcií |
| Notes | [log](NOTES-LAB-LOG.md) | 10 návrhov, 12 funkcií |
| Pulls | [log](PULLS-LAB-LOG.md) | 10 návrhov, 12 funkcií |
| Events | [log](EVENTS-LAB-LOG.md) | 20 návrhov, 12 funkcií |
| Sales / Inventory | [log](SALES-INVENTORY-LAB-LOG.md) | 15 návrhov, 12 funkcií |

Spolu **155 návrhov a 91 funkcií**, všetky preklikateľné.

---

## 7. ČO ĎALEJ

Nič z tohto zatiaľ **nie je v produkcii**. Laby sú náhľady.

Poradie, ktoré dáva zmysel:

~~1. Tri chyby v Settings~~ — **hotové, 2.66.0.** Viď
`REDESIGN-2.66.0-REPORT.md`.

Ostáva teda len samotná prestavba, ktorá nikam nehorí:

1. Vzhľad — register na dashboard, inventory, sales, notes
2. Funkcie podľa toho, čo si vybral

**Nič z toho nie je skompilované.** Na tomto stroji nie je npm ani cargo,
takže 2.65.0 aj 2.66.0 uvidí prvýkrát až CI.

*(Mixed Money bolo v skorších verziách tohto dokumentu na prvom mieste.
Bolo to moje nedorozumenie — viď 3.1.)*

### Vybrané funkcie, zhrnuté

| Oblasť | Funkcia | Čo ju robí bezpečnou |
|---|---|---|
| Dashboard | Bankový výpis cez CSV | — |
| Settings | Import Preview | nič sa nezapíše, kým nevidíš rozdiel |
| Notes | Pin to Record · Quick Note · Real Checklist · Note Templates | — |
| Inventory | **Restriction Stamp** | dry run odmietne prepísať ručne upravený lístok |
| Oboje | **Bulk Change Preview** | ukáže, čo by sa stalo, a vie to vrátiť |

Tri z piatich sú **náhľad pred zápisom**. To je vzor, nie zhoda náhod:
všetko, čo sa dotýka viacerých riadkov naraz, ti to má najprv ukázať.
