# TIQR Manager 2.50.1 — sync sa už nespúšťa pri každom kliknutí na okno

Toto je moja regresia a mal si pravdu.

---

## Čo sa stalo

V **2.48.1** som pridal, že sa sync spustí aj vtedy, keď sa vrátiš na okno
appky. Robil som to preto, že si predtým písal, že prepnutie medzi počítačmi
nie je dosť rýchle — a s tým to bolo okamžité.

Lenže fungovalo to až príliš doslova. **Každé alt-tabnutie, každé kliknutie
späť na okno = sync** — a zakaždým to o sebe dalo vedieť v hlavičke. Presne
ako píšeš: odídeš na pár sekúnd, vrátiš sa, a ono zase.

## Čo je teraz

Oba tie spúšťače sú **preč**.

Zostáva:

- **jeden sync pri spustení appky**,
- a potom **ticho na pozadí každých 5 minút**.

## Prečo som nechal ten päťminútový časovač

Keby som zrušil aj ten, vrátila by sa presne tá vec, na ktorú si sa sťažoval
o dve verzie skôr: že sa dva počítače **nespoja**, keď necháš appku otvorenú.

Na rozdiel od toho focusu je ale **neviditeľný** — keď nie je čo preniesť,
neurobí nič a nič nenapíše. Uvidíš ho len vtedy, keď naozaj niečo ide hore
alebo dole.

**Ak chceš doslova len pri spustení a nič viac**, poviem a zruším aj ten
časovač — ale potom sa dva otvorené počítače nezrovnajú, kým jednu z appiek
nezavrieš a neotvoríš.

---

## Čo som overil

- **Oba spúšťače sú naozaj preč** — po zmene nezostal v kóde ani jeden
  `focus`/`visibilitychange` odber, len komentár, ktorý vysvetľuje prečo.
- **Zátvorky v `Layout.tsx`** proti 2.49.2 — **bez posunu**.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú).

---

**Verzia:** 2.50.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
