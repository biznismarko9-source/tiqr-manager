# TIQR Manager 2.49.0 — dátum sa dá písať, mena sa vyberá

---

## 1. Dátum

**Ono to nebolo pokazené — ono sa doň nikdy nedalo písať.**

To políčko bolo v skutočnosti **tlačidlo**. Vyzeralo ako políčko, ale nebolo
v ňom nič, do čoho by sa dalo klepať; jediné, čo vedelo, bolo otvoriť kalendár.
Takže keď si klikol a začal písať číslice, nemalo to kam ísť.

### Ako to funguje teraz

Je to **normálne textové políčko**. Klikneš, píšeš číslice a **bodky si doplní
samo**:

| Píšeš | Ukáže |
|---|---|
| `2` | `2` |
| `2109` | `21.09` |
| `21092026` | `21.09.2026` |

**Kalendár zostáva** — je to tá ikonka vpravo priamo v políčku. Obe cesty sú
rovnocenné, ani jedna nie je druhoradá.

### Čo neprejde

**Dátum, ktorý neexistuje.** `31.02.2026` je osem úplne správnych číslic a aj
tak to nie je deň — appka ho neprevezme. Priestupný rok sedí: **29.02.2024 áno,
29.02.2026 nie**. Nekontrolujem to tabuľkou dní v mesiaci, ale tak, že ten
dátum naozaj poskladám a pozriem sa, či vyšiel ten istý — takže priestupné roky
nemá kto pokaziť.

**Rozpísané `21.0` sa do formulára nedostane.** Žije len v políčku, kým z neho
nie je celý dátum. Keď odklikneš preč bez dokončenia, vráti sa posledná platná
hodnota — nič sa nezmaže a nič nezostane rozbité.

## 2. Mena sa vyberá zo zoznamu

Pri **order, sale aj pull** je mena rozbaľovačka s tými istými **13 menami**,
aké už ponúka zvyšok appky (Orders aj detail eventu) — jeden zoznam na jednom
mieste, nie tri kópie.

**Keby si mal niekde menu mimo zoznamu**, zostane ti a bude prvá v ponuke.
Appka ti ju ticho neprepíše na EUR — to by bola chyba v peniazoch prezlečená za
opravu vzhľadu.

---

## Čo som overil

- **Písanie dátumu som prepísal do Pythonu a spustil na 16 prípadoch** —
  neúplné vstupy, bodky navyše, písmená, prebytočné číslice, neexistujúci deň,
  13. mesiac, nula ako deň a oba priestupné prípady. Všetky sedia.
- **Zátvorky** vo všetkých štyroch zmenených súboroch proti 2.47.5 — **bez
  posunu**.
- **Rozdiel v značkách je presne ten očakávaný**: o jeden `Input` menej
  v každom z troch formulárov (mena sa stala `Select`) a jeden nový input
  v `ui.tsx` (to políčko na dátum).
- **Žiadny nepoužitý import** — `Input` aj `Select` sa v každom súbore ďalej
  používajú.
- **Každá cesta, ktorá nastaví dátum, vyčistí rozpísané** — deň v kalendári,
  Today aj Clear. Overené, že som žiadnu nevynechal.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a ako presne ti sadne
písanie do políčka pri reálnom klepaní.

---

**Verzia:** 2.49.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
