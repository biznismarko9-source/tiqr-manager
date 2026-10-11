# TIQR Manager 2.89.0 — report

**Čo si povedal:** „tie zmeny vobec neboli velke, som sklamany, tie okienka.
vsetky ostali rovnake, totreba zmenu, chcem ich tmavsie a taka krajsie"

Mal si pravdu. A nebolo to tým, že by som sa málo snažil — bolo to tým, že
som menil veci **na** okienkach, kým problém bol v samotnom okienku.

---

## 1. Prečo všetky ostali rovnaké

**Karta bola svetlejšia než stránka.**

| | hodnota |
|---|---|
| podklad stránky | `#0b0b0c` |
| karta | `#141415` ← svetlejšia |

Takže každá karta v appke bola ten istý stredne sivý obdĺžnik, ktorý sa
odlepoval od pozadia. Nech som v nej vypol graf alebo krúžok, **okienko
zostalo to isté okienko.** To si videl.

## 2. Čo je teraz

| | pred | po |
|---|---|---|
| podklad stránky | `#0b0b0c` | **`#050507`** |
| karta | `#141415` | **`#0c0c10`** |
| hrana karty | `#1f1f23` | **`#32323d`** |
| horná hrana | — | **svetlo, 96 px nadol** |

Karta je teraz zhruba tam, kde bolo **staré pozadie stránky**, a pozadie
kleslo ešte hlbšie pod ňu.

Keď výplň prestane kresliť kartu, musí ju nakresliť niečo iné — preto je
hrana svetlejšia než bola, a preto má karta **svetlo po hornej hrane**. To je
tá časť, čo robí „krajšie": okienko prestane byť plochý obdĺžnik a začne mať
hranu, ktorá chytá svetlo.

To svetlo je pevných **96 px**, nie percento. Dôvod: vysoký panel s tabuľkou
tak dostane presne tú istú hranu ako nízka KPI dlaždica, namiesto prechodu
natiahnutého cez celú výšku.

---

## 3. Prečo sa to nedalo spraviť „len zmenou jednej farby"

Tri veci stáli v ceste a každú bolo treba vyriešiť zvlášť:

**`--surface` nemohol ďalej ukazovať na `slate-900`.** Ten istý stupeň farby
nesie **text v light móde** (`text-slate-900`). Keby som hýbal rampou, zmením
ti farbu písma v svetlom režime. Karta preto dostala vlastnú hodnotu.

**`slate-950` sa zmeniť dal** — ale až potom, čo som si overil, že v celej
appke neexistuje ani jeden `text-slate-950`, `border-slate-950` ani
`ring-slate-950`. Nesie výhradne pozadie stránky, takže sa mohlo stmaviť
celé naraz.

**20 miest malo starú farbu napísanú natvrdo.** `dark:bg-slate-900` priamo
v kóde, vedľa `bg-surface`. Kým sa token rovnal `slate-900`, boli neškodné —
ale po zmene by to bolo **20 svetlejších fľakov**, čiže doslova to, na čo si
sa sťažoval. 17 z nich override stratilo a ide za tokenom. 3 majú
priehľadnosť (`/60`, `/90`, `/95`), čo Tailwind cez premennú nevie spracovať,
takže ukazujú na hodnotu priamo.

**Čo som zámerne NEHÝBAL:** `--line`. Niekoľko stoviek miest v appke píše
`dark:border-slate-800` natvrdo. Token, ktorý by s nimi nesúhlasil, by ti
spravil dve rôzne vlásočnice na jednej obrazovke.

---

## 4. Ako som vybral práve toto

Nehádal som. Postavil som **päť variantov** s tvojimi vlastnými číslami
a pozrel som sa na ne vedľa seba:

- **A** — hlbšia čierna, ostrejšia linka
- **B** — zapustené okienka (pozadie svetlejšie, karta zapustená dnu)
- **C** — onyx s horným svetlom
- **D** — plochá, drží ju len ostrá linka

Vybral som C, potom som ju porovnal ešte s verziou o stupeň hlbšou — a vzal
tú hlbšiu. Po minulom kole som nechcel riskovať, že to zasa bude kozmetika.

**Priložený obrázok `2.89.0-pred-a-po.png`** je render pred/po s presne tými
hodnotami, čo sú v kóde. Pozri sa naň skôr, než to nainštaluješ.

Ak chceš ešte tmavšie, alebo radšej variant B (okienka zapustené dnu namiesto
položené navrchu), povedz — mám ich pripravené.

---

## 5. Overenie

Vyrenderované s tvojimi číslami, s hodnotami skopírovanými priamo z kódu.
Plus vyváženie zátvoriek vo všetkých TS súboroch aj v `index.css`
a `tailwind.config.js`, a obe triedy komentárovej chyby (0 výskytov).

**Nie je to kompilácia.** Na tomto Macu nie je node ani cargo — CI je prvý
kompilátor. Platí ako vždy.

---

## 6. Verzia

2.88.0 → **2.89.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
