# TIQR Manager 2.43.0 — bočný panel je plochý

Skupina **Tickets** z panela odišla. **Events, Inventory, Sales a Pulls sú
vždy vidieť** — nič sa neotvára, nič sa nezatvára, nič sa neskrýva.

---

## Ako to teraz vyzerá

Šesť riadkov v poradí, v akom práca beží:

**Dashboard · Events · Inventory · Sales · Pulls · Finance**

Settings zostáva dole nad profilom, presne kde bol.

Finance som zámerne nechal ako posledné a **nezaradil ho k tým štyrom** — je
to o peniazoch, ktoré s lístkami nemajú nič spoločné (nájom, poplatky, osobné
výdavky) a má vlastné štyri karty, kategórie a účty.

## Čo sa upratalo popri tom

S rámčekom zmizol aj:

- rozbaľovací stav panela,
- výpočet „som niekde v tejto skupine",
- nepoužitý typ riadku-nadpisu, ktorý nemal ani jednu položku **od 2.36.0**,
- dve ikony, ktoré už nemal kto použiť.

Vykresľovanie panela je teraz **jedna vetva namiesto troch**.

## Čím to zároveň skončilo

Pamätáš tú otázku, či sa má skupina zvýrazniť spolu s položkou? V 2.29.4 si
chcel, aby sa zvýrazňovala, v 2.41.0 aby nie. **Teraz je bezpredmetná —
skupina neexistuje.** Obe rozhodnutia nechávam zapísané v
`PROTECTED_AREAS.md` ako históriu, ale s poznámkou, že už nepopisujú živý kód.

---

## Čo som overil

- **Vyváženosť značiek** oproti balíku 2.42.0 — bez nepáru.
- **Žiadny nepoužitý import** v `Layout.tsx` (strojovo; `IconTicket` a
  `IconChevronDown` sa stali nepoužitými a sú preč).
- **Všetkých šesť ciest** je v paneli: `/`, `/events`, `/orders`, `/sales`,
  `/pulls`, `/finance` — plus `/settings` dole.

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a ako to sadne
oku. Ak ti bude šesť riadkov za sebou pripadať dlhých, viem medzi ne vrátiť
tenkú čiaru — to je jeden riadok CSS, nie návrat skupiny.

---

**Verzia:** 2.43.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
