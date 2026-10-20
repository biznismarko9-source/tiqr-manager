# TIQR Manager 2.47.2 — vypĺňovač je širší

> „tuto cast kde sa to vyplna kludne urobme sirsiu nech sa tam vojde viac info
> a aby bolo to info vidiet cele"

Na tvojej fotke to bolo presne vidieť: okno sedelo uprostred oveľa širšieho
monitora, **Typ ukazoval len „—"** a Sektor s Radom boli užšie než ich vlastné
nadpisy.

---

## Čo bolo zle

Okno malo **napevno 1152 px** (`max-w-6xl`). To číslo nemalo nič spoločné
s tvojou obrazovkou — bolo rovnaké na notebooku aj na veľkom monitore. Takže
na širokom displeji ostalo po oboch stranách prázdno a stĺpce sa tlačili.

## Čo som spravil

**Okno je teraz až 1560 px** — respektíve 94 % šírky okna, keď máš menšie.
Berie miesto, keď je, a stále sa zmestí na notebook (tam sa tabuľka odroluje
nabok ako doteraz).

**Rozpočet na stĺpce narástol z 1112 na 1520 px** (28 na číslo riadku, 1428 na
dáta, 64 na ikonky). Najviac dostali tie, čo boli najstlačenejšie:

| Stĺpec | Predtým | Teraz |
|---|---|---|
| Typ | 110 px | **150 px** |
| Sektor | 92 px | **130 px** |
| Rad | 64 px | **90 px** |
| Sedadlá | 108 px | **150 px** |
| Platforma | 130 px | **190 px** |
| Cena/ks | 96 px | **120 px** |
| Mena | 70 px | **90 px** |
| Pull | 152 px | **200 px** |
| Poznámka | 126 px | **230 px** |

Rovnako narástli aj **pulls, sales a events** — je to ten istý formulár.
Napríklad Názov pri evente 232 → 330 px a Event pri predaji 236 → 360 px.

Výber eventu a „Poznámka k celému nákupu" hore sa roztiahnu samé, lebo idú cez
celú šírku okna.

---

## Čo som overil

- **Všetky štyri sedia na rozpočet**, strojovo spočítané: order 1428, pull
  1428, event 1428, sale 1370 (má menej stĺpcov) — plus 28 a 64.
- **Počet hlavičiek = počet buniek** vo všetkých štyroch. To je kontrola, ktorá
  by odhalila, keby som pri prerozdeľovaní nejaký stĺpec stratil alebo pridal.
- **Zoznam súborov** oproti 2.47.1: **0 pribudlo, 0 zmizlo, 4 zmenené.**
- **Zátvorky** proti 2.47.1 — identické.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a ako presne to sadne
na tvojom monitore. 1560 px je môj odhad — ak to chceš ešte širšie alebo naopak
užšie, je to jedno číslo na štyroch miestach a viem to hneď prehodiť.

**Pripomínam tú otázku z 2.47.1:** stále potrebujem vedieť, čo ti vypíše AI
import, keď ho skúsiš — „AI import isn't available in this build." (chýbajúci
kľúč v builde, nie je to oprava v kóde) alebo „AI analysis failed. Try again."
(vtedy to viem rozobrať ďalej).

---

**Verzia:** 2.47.2 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
