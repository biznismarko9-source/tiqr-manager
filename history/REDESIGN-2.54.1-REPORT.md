# TIQR Manager 2.54.1 — oprava buildu

**Obsahovo je to presne to isté ako 2.54.0** (Sheets — naozajstná mriežka).
Toto je len oprava toho, že sa 2.54.0 nezostavilo.

---

## Čo sa pokazilo

```
src/pages/sheets/Grid.tsx(424,35):
error TS2552: Cannot find name 'isEditing'. Did you mean 'editing'?
```

Moja chyba. Premennú `isEditing` som v mriežke premenoval na `cellEdit` a
**jeden jediný výskyt mi ušiel**. TypeScript to zastavil a build padol na
Windows aj na Macu.

Jednoriadková oprava.

---

## Prečo to moje kontroly nechytili

Mám kontroly na zátvorky, na importy, na to, či sa každý príkaz medzi appkou a
Rustom páruje. **Všetky boli zelené** — a ani jedna z nich takúto chybu vidieť
nevie. Zátvorky sedia, import je v poriadku, len sa používa meno, ktoré už
neexistuje.

Tak som si dopísal kontrolu, ktorá presne toto chytá: vypíše mená premenných,
ktoré sa v súbore vyskytujú **práve raz**. Zvyšok po premenovaní vyzerá presne
tak. **Overil som ju tak, že som tú chybu naschvál vrátil späť — našla ju** —
a potom som ju pustil na všetky zmenené súbory. Čisté.

---

## A ešte niečo dôležitejšie, čo z toho vyliezlo

Build púšťa **najprv TypeScript a až potom Rust**. Keďže TypeScript padal,
**Rust sa nikdy ani nezačal kompilovať** — ani teraz, ani v 2.52.0, ani v
2.53.0.

Čiže ten Rust, čo som ti posielal posledné tri verzie, **nikdy neprešiel
kompilátorom**. Tak som ho prešiel a dve miesta prepísal tak, aby nemohli
robiť problém:

1. **Zbaľovanie výsledkov hľadania** — bolo tam napísané spôsobom, ktorý Rust
   pri práci s pamäťou neprepúšťa (vraciala sa referencia zvnútra uzáveru).
   Prepísal som to natvrdo, bez tej skratky.
2. **Presúvanie stĺpcov** — spoliehalo sa na to, že sa transakcia zahodí v
   správnej chvíli, aby sa uvoľnil prístup k databáze. Teraz sa obe situácie,
   keď sa nič nemá diať, vyriešia **skôr, než transakcia vôbec vznikne**.

---

## Čo som overil

- **1 oprava**, plus dve preventívne úpravy v Ruste
- kontrola na zvyšky po premenovaní — **overená na tej istej chybe**, potom
  čistá na všetkých zmenených súboroch
- **všetkých 34 SQL príkazov** (8 vo workspace.rs, 26 v notes.rs) som **pustil
  proti skutočnej databáze** postavenej zo všetkých 32 migrácií — všetky
  prešli, vrátane toho jedného skladaného za behu
- zátvorky, importy, exporty, 20 príkazov 1:1, 204 volaní — zelené
- migrácie sa nemenili, databáza sa nemenila

**Neoverené:** stále nemám Node ani Rust, takže appku nezostavím. Ale tentoraz
som prešiel aj ten Rust, nie len TypeScript.

---

*2.54.0 už nepoužívam ako číslo — CI naň už raz bežalo. Preto 2.54.1.*
