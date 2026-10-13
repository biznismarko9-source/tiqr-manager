# TIQR Manager 2.92.0 — Premium Noir + prísny audit

Žiadal si dve veci: transformáciu podľa referencie, a **prísny audit toho, čo
som naozaj zmenil** oproti tomu, čo som len preštýloval.

Audit robím prvý, lebo jeho výsledok ma prekvapil a mení, čo bolo treba robiť.

---

## 1. AUDIT — obrazovka po obrazovke, z kódu

| Obrazovka | Štruktúra dnes | Verdikt |
|---|---|---|
| **Dashboard** | riadok čísel = jeden panel delený linkami (2.90.0), graf, platformy, aktivita | **prestavané** |
| **Events** | *The Table*: deň pod dátumom, mono čísla, hlásenie zmiešanej meny | **prestavané** |
| **Event detail** | *One Page* — záložky Overview/Sales zrušené (2.90.0) | **prestavané** |
| **Notes** | `Notes.tsx:212` — 300 px zoznam + `flex-1` editor | **už bolo dvojpanelové** |
| **Settings** | `Settings.tsx:890` — `lg:w-56` ľavý sub-nav + obsah | **už malo sub-nav** |
| **Inventory** | lišta filtrov s popiskami + segment + `table-shell` tabuľka | **už má architektúru referencie** |
| **Sales** | to isté | **už má architektúru referencie** |
| **Pulls** | to isté + `SEGMENTED_TRACK` | **už má architektúru referencie** |
| **Finance Overview** | riadok čísel (2.90.0) + grafy | **prestavané** |
| **Finance Transactions / Accounts / Reports** | nedotknuté | **ostáva** |
| **Dialógy** | neauditované | **ostáva** |

### Čo z toho plynie

Čakal som, že budem prestavovať Notes a Settings. **Nebolo treba** — obe sú
už presne tak postavené, ako to ukazuje referencia. Notes má dvojpanel,
Settings má ľavý sub-nav. Inventory, Sales aj Pulls majú lištu filtrov nad
čistou tabuľkou, čo je tiež architektúra z referencie.

Tie obrazovky teda nevyzerali staro **štruktúrou**. Vyzerali staro
**vizuálnym jazykom** — a ten dostali v tomto vydaní.

Neprestavoval som ich nasilu len preto, aby diff vyzeral veľký. Mandát sám
hovorí: *„Do not use the amount of code changed as the measure of success."*

---

## 2. Čo sa zmenilo: Premium Noir

**Akcent violet → tyrkys**, celá rampa `brand`.

Jeden dôsledok nie je kozmetický: **plnené tlačidlo už nemôže niesť biely
text.** Biela na `brand-600` meria **3,0 : 1** — pod hranicou AA. Primárne
tlačidlo je teraz **takmer čierny text na `brand-400`** (9,4 : 1), čo je
zároveň presne to, čo ukazuje tvoja referencia.

**Povrchy — chladná čierna:**

| | pred | po |
|---|---|---|
| podklad | `#050507` neutrálny | **`#080b0e`** chladný |
| karta | `#0c0c10` | **`#0e1317`** |
| linka | `#32323d` | **`#1e262e`** |
| zapustené | `#020203` | **`#05080a`** |

Ten modrastý nádych je väčšina toho, čo oddeľuje „premium noir" od
„tmavosivej appky".

**Navigácia:** aktívna položka nesie akcent **aj v texte**, nielen vo výplni.
Samotná výplň pri šírke 192 px čítala ako „prejdené myšou". Settings sub-nav
dostal ten istý stav — sú to jediné dva navigačné raily v appke a nesmú sa
rozísť.

---

## 3. Konflikt, ktorý musím povedať nahlas

**Tvoja referencia a THE REGISTER si odporujú.**

Register (`REDESIGN-OVERVIEW.md` §1, tebou uzavretý 30.09.) hovorí: violet,
**jeden akcent len na výber**, žiadne tiene, prechody ani ilustrácie.

Referencia je tyrkysová, má sparkliny, avatary interpretov a bohatšie
povrchy.

Vlastník si ty a tento smer si vybral písomne, takže som šiel podľa
referencie. Kde sa zhodujú — vlasové linky, 8 px panely, popisky nad
ovládačmi, mono čísla — tam práca z 2.90.0 ostáva nedotknutá.

**Sparkliny som nevrátil**, hoci sú v referencii: v 2.88.0 si mi výslovne
kázal ich odstrániť. Tvoj priamy pokyn má prednosť pred obrázkom.

---

## 4. Čo som ZÁMERNE nepostavil

**Globálne hľadanie v hornom pruhu.** Referencia ho má, appka ho nemá. Nemám
pre neho backend a postaviť ho ako atrapu je presne to, čo mandát zakazuje:
*„Do not introduce decorative controls that appear functional but do
nothing."*

**Avatary interpretov v tabuľkách.** Appka nedrží žiadnu fotku interpreta.
Domyslieť si ju — alebo ťahať ju odniekiaľ z internetu — je presne to, čo
táto appka nesmie nikdy robiť.

Obe sú riešiteľné, ale sú to **funkcie**, nie vzhľad. Ak ich chceš, povedz
a postavím ich poriadne.

---

## 5. Čo ešte ostáva

- Finance **Transactions / Accounts / Reports** — ešte nedostali štrukturálny
  prechod
- **Dialógy a sekundárne obrazovky** — neauditované
- Bankový výpis cez CSV · Restriction Stamp · Bulk Change Preview ·
  Import Preview — nepostavené (všetko „náhľad pred zápisom", siahajú na
  migrácie a peniaze)

---

## 6. Overenie

| Kontrola | Výsledok |
|---|---|
| Render: shell, nav, tlačidlá, čísla, tabuľka v novej palete | pozreté |
| Vyváženie zátvoriek, celý `src/` | OK |
| `index.css` 108/108 · `tailwind.config.js` 12/12 | OK |
| Komentárové chyby A + B | 0 |

**Nie je to kompilácia.** Node ani cargo na tomto Macu nie sú — CI je prvý
kompilátor, a dev server sa z toho istého dôvodu spustiť nedá.

---

## 7. Verzia

2.91.0 → **2.92.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
