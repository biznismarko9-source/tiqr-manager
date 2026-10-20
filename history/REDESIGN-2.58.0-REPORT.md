# TIQR Manager 2.58.0 — Poznámky

Vybral si si **návrh 02** a počas stavby si dopísal tri veci. Všetky tri sú
vnútri.

---

## Každý riadok má svoje vlastné nastavenie

> *„vies si vybrat kde chces mat fotku, kde chces pisat, kde ten text ma byt
> vacsi, aka farba"*

Toto bolo to hlavné rozhodnutie. Poznámka **nie je jeden veľký text** — je to
zoznam riadkov a **každý riadok má svoje**:

| | |
|---|---|
| **Veľkosť** | veľký nadpis · nadpis · text · malý text |
| **Farba** | 6 farieb + základná |
| **Štýl** | tučné, kurzíva |
| **Checkbox** | z hociktorého riadku spravíš odškrtávacie políčko |
| **Obrázok** | vložíš presne tam, kde stojíš |
| **Čiara** | na oddelenie |

Klikneš na riadok, lišta hore platí **pre ten riadok**. Enter spraví nový — a
ak si bol na checkboxe, nový je zase checkbox.

---

## Podkarty

> *„v jednej karte si vies pridat podkarty niekde dole ako to je v google
> sheets ze napr budes mat poznamku oasis kody a vies si povyberat podkarty"*

Dole v poznámke je **pás podkariet**, presne ako taby v Sheets.

Poznámka **„Oasis kódy"** → podkarty `ABC123`, `DEF456`, `GHI789`… a v každej
si píšeš o tom kóde zvlášť.

**+** pridá · **dvojklik** premenuje · **×** zmaže (posledná sa zmazať nedá,
poznámka vždy má aspoň jednu).

---

## Priradenie k čomukoľvek

> *„neni to len pre kody ale aj do buducna na zapisovacky, priradovanie eventu,
> pullu, inventaru, sellu atd"* + *„aj finance"*

Tlačidlo **⧉ Priradiť**. Vyberieš **typ** a potom konkrétny záznam:

**Objednávka · Event · Inventár · Predaj · Pull · Financie**

Pripojíš ich aj **viac naraz**. Zobrazia sa ako štítky v poznámke aj v zozname.

**Keď ten záznam neskôr zmažeš, poznámka ostane** — zmizne len prepojenie. To
je zámerné: poznámku, čo si k tomu napísal, budeš chcieť aj tak.

### Prečo to spomínam

Skoro som to spravil jednoducho ako „typ + číslo". To by bolo **ticho zlé**:
pri zlučovaní dvoch počítačov appka prekladá čísla záznamov podľa toho, na
**ktorú tabuľku** stĺpec ukazuje. Pri univerzálnom „typ + číslo" by nemala
podľa čoho prekladať a poznámka by sa po zlúčení mohla pripnúť k **úplne inej
objednávke**. Preto je tam šesť samostatných stĺpcov — a overil som, že
poradie zlučovania je správne.

---

## Obrázky

Vyberieš súbor a appka ho **sama zmenší** (najdlhšia strana 1400 px, JPEG).

Robím to preto, že sync nahráva **celú databázu**. Jeden 6 MB screenshot z
mobilu by sa tak platil znova a znova pri každom syncu. Po zmenšení je to
rádovo desiatky až stovky kB.

---

## Ukladá sa samo

Chvíľu po písaní, pri prepnutí podkarty a pri odchode z poznámky. Dole vpravo
vidíš `Uložené` / `Ukladám…`.

---

## Staré hárky (Sheets)

**Nič som nezmazal.** Sheets sú stále v appke a stále sa synchronizujú — len
už nie sú v ľavom paneli, miesto nich sú Poznámky.

Dole v zozname poznámok máš odkaz **„Staré hárky (N)"**, kým tam nejaké máš.
Presúvaj si to vlastným tempom, alebo mi povedz a spravím s tým, čo chceš.

---

## Čo v tomto vydaní NIE JE

**Sync v pozadí**, ktorý si pýtal — aby ti appka fungovala, kým sa syncuje, a
nemusel si čakať na načítanie.

Nedal som to sem naschvál. Sync je chránená časť a na konci takto veľkej roboty
by som ho lepil narýchlo. Spravím ho ako samostatnú vec hneď ďalej.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 36 migrácií** na čistej databáze
- **správanie schémy**: zmazanie objednávky nechá poznámku a odstráni len
  prepojenie; zmazanie poznámky vezme podkarty, obrázky aj prepojenia; všetky
  štyri tabuľky píšu stopy pre synchronizáciu
- **poradie zlučovania**: žiadna tabuľka sa nezlučuje skôr ako tá, na ktorú
  ukazuje (`note_links` je posledná, po všetkých šiestich)
- **34 SQL príkazov** v novom module proti skutočnej schéme
- **51 príkazov Rust↔appka 1:1**, 235 volaní registrovaných
- **každé pole**, ktoré čítam z objednávky/eventu/inventára/predaja/pullu/
  financií, naozaj v tých typoch existuje — tu som našiel tri chyby (`soldDate`
  neexistuje, `Pull` nemá `status`, `listFinanceEntries` neberie parameter) a
  opravil ich
- zátvorky, importy, kontrola na zvyšky po premenovaní — čisté

**Neoverené:** appku nezostavím (nemám Node ani Rust) — to spraví tvoj build.
Naklikať som to tiež nevedel. Keby čokoľvek nesadlo, pošli screenshot.
