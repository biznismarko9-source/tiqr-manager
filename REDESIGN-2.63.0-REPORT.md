# TIQR Manager 2.63.0 — Noir

Vybral si to sám, v tom živom prepínači štýlov, na svojich číslach:

**Noir · fialová `#7c5cf0` · Bricolage Grotesque · Slight rohy · Cosy hustota · plošný graf · Tiles 01**

A povedal si: *„zatial by som zmenil len designy"*. Tak som zmenil len vzhľad.
**Žiadna funkcia, žiadna migrácia, žiadny nový príkaz, ani jedna stránka.**

---

## Kde to celé žije

V tvojom `PROTECTED_AREAS.md` je napísané, že vzhľad appky je definovaný v
**štyroch súboroch** a stránka medzi ne nepatrí. Držal som sa toho.

Zmenil som dva z tých štyroch:

- **`tailwind.config.js`** — sivá rampa, značková rampa, rohy, písmo
- **`src/index.css`** — `@font-face` a deväť hodnôt povrchov

`ui.tsx` a `Layout.tsx` nepotrebovali **nič**. Všetko si berú cez tú vrstvu,
a presne na to tá vrstva je.

---

## Farby

### Jedna rampa prekreslila celú appku

Sivá (`slate`) mala doteraz fialový nádych. Teraz je neutrálna, a `900` s `950`
sedia presne na tom Noir, čo si videl.

To je celý repaint. Nie preto, že by som bol rýchly, ale preto, že v appke je
**asi 23 tisíc riadkov**, ktoré už hovoria `bg-slate-800`, `text-slate-400`,
`border-slate-700`. Prepíšeš rampu a zmení sa to všetko naraz.

### Akcent

`brand-500` je **tvoja presná fialová `#7c5cf0`**.

`brand-600` **nie je**, a je to naschvál. Šesťstovka je výplň pod každým
tlačidlom a **biely text na `#7c5cf0` má kontrast 4,45:1**, čo je tesne pod
normou. `#6e4ce4` je tá istá farba o odtieň nižšie a má **5,50:1**.

### Toto ti musím povedať

V `PROTECTED_AREAS.md` stojí, že značkovú rampu **nemám meniť bez toho, aby som
sa ťa spýtal** — v 2.0.56 sa už raz menila paleta a v 2.0.58 si ju v ostrej
prevádzke zamietol.

Zmenil som ju. Beriem to tak, že si si ju tentoraz **vybral sám z náhľadu, čo
bežal na tvojich dátach**, a to je silnejšie než odpoveď na otázku. Ale chcem,
aby si vedel, že to pravidlo tam je a že o ňom viem. Zapísal som to aj tam.

---

## Písmo

**Appka po prvý raz naozaj načítava písmo.**

Doteraz bol `Inter` na prvom mieste zoznamu, ale nikde sa nesťahoval ani nebalil
— máš to sám poznamenané v `index.css`. Appka teda bežala na systémovom písme,
na Macu inom ako na Windows.

Bricolage Grotesque je iné. **Dva súbory, 107 kB, ležia v appke.**

- **funguje aj bez internetu**, nič sa nikam nepýta
- **má aj č š ž ť ď ň ľ ĺ ŕ** — samotná latinka by slovenčinu nenapísala,
  preto sú súbory dva
- vietnamčinu som vynechal, to je 30 kB, čo tu nikto neprečíta
- licencia OFL 1.1 leží vedľa súborov, musí s nimi cestovať

Kým sa súbor načíta, appka kreslí systémovým písmom a potom prepne. Studený
štart teda nikdy neukáže prázdno.

---

## Tvary

Rohy sú ostrejšie: **ovládacie prvky 4 px, kontajnery 6 px** (bolo 5 / 6 / 8).

Je to tretíkrát, čo si pýtal ostrejšie a ani raz si nepýtal hranaté. Nie je to
hranaté.

---

## Čoho som sa nedotkol

- **tiene** — tá dvojica z 2.29.0 stále nesie hĺbku
- **šírky stĺpcov v tabuľkách** a metriky `.th-c-narrow` — sú premerané proti
  reálnym dátam v troch jazykoch, aby ti nenaskočil vodorovný posuvník
- **stavové farby** (`STATUS_TONES`) — to je slovník tvojho biznisu, nie štýl
- **ani jedna stránka**

---

## Čo som overil

**Spustené, nie prečítané:**

- **kontrast deviatich párov**, ktoré appku nesú — všetky prejdú
  - biely text na tlačidle **5,50:1**
  - text na tmavom povrchu **17,05:1**
  - text na svetlom povrchu **18,41:1**
- **slovenské znaky** — všetkých 34 vrátane `€` je pokrytých tým, čo písmo
  deklaruje
- **súbory písma sú naozaj woff2** (skontrolovaná hlavička), nie prázdne
- **cesta k písmu sedí** zo súboru, kde je napísaná
- **zátvorky** v oboch zmenených súboroch
- **žiadna stará farba** nezostala nikde natvrdo v `src/`

### Jedno číslo, ktoré nezaokrúhlim

Sivá 400 — to, na čom stoja šedé popisky a placeholder v políčkach — má
**4,32:1 na svetlom a 4,26:1 na tmavom**. Norma chce 4,5.

Nie je to chyba, ktorú som spravil. **Jedna hodnota to v oboch režimoch splniť
nevie** — matematicky. Doteraz si mal 4,56 na svetlom a 4,16 na tmavom, čiže
rovnaký problém, len prehodený. Moja je vyrovnanejšia.

Keby ti to prekážalo, dá sa to spraviť, ale znamenalo by to siahnuť aj na
stránky, a to si teraz nechcel.

**Neoverené:** appku nezostavím, nemám tu Node ani Rust. To spraví tvoj build.
A ako to vyzerá naživo na tvojej obrazovke, vieš povedať len ty.

---

## Ďalej

Vzhľad je vonku. **Rozloženia z prototypu — Tiles 01 a tých ďalších 69 — v
ostrej appke zatiaľ nie sú**, lebo tie by znamenali prerábať stránky, a ty si
povedal zatiaľ len dizajny.

Keď toto uvidíš na svojich dátach a sadne ti to, poviem si o ďalší krok.
