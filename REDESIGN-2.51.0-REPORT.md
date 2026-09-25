# TIQR Manager 2.51.0 — Notes: miesto, na ktoré sa dá spoľahnúť

Plus oprava toho syncu z tvojej prvej správy (tá vyšla zvlášť ako 2.50.1).

---

## Čo to je

Nová položka v paneli: **Notes**. Hárky, ktorým si **sám pomenuješ stĺpce** —
v princípe Google Sheets, ale vnútri appky a synchronizované medzi oboma
počítačmi.

Vybral si si to zo troch možností a k tomu, že zápisky stoja samostatne (nie
pripnuté k eventu či objednávke).

## Ako to funguje

Vytvoríš hárok, píšeš riadky. Stĺpce si kedykoľvek **pridáš, premenuješ alebo
zmažeš**.

**Každá bunka sa uloží sama**, hneď ako z nej klikneš preč. Žiadne tlačidlo
Uložiť — takže nikdy nie je stav, v ktorom si niečo napísal a ono to nie je
uložené. To je celé to „miesto, na ktoré sa dá spoľahnúť".

## Aby to nebol prázdny papier

Nový hárok vieš začať zo **šablóny** — priamo z toho, čo si písal:

| Šablóna | Stĺpce |
|---|---|
| **Buyers** | Nick · Ticket code · Event · Paid · Contact · Note |
| **Accounts** | Platform · Account · Email · Note |
| **Plans** | What · By when · Status · Note |
| **Blank** | jeden stĺpec, pomenuj si ho |

Sú to len začiatky — hárok je potom tvoj a stĺpce si prerobíš.

## Nájdeš v tom všetko

Hore je **jedno hľadanie cez všetky hárky naraz**. Napíšeš nick, kód lístka
alebo kus mena a vypíše ti to riadok — **v ktorom hárku** je a **v ktorom
stĺpci** sa to našlo. Klikneš a si tam.

Naschvál je to obyčajné hľadanie podreťazca, nie „inteligentné": keď hľadáš
`TKT-88`, chceš nájsť aj `TKT-8801`. Sofistikovanejšie indexovanie by ti to
práve nenašlo.

## Synchronizuje sa — a to bola tá skrytá polovica roboty

Zápisky idú medzi Macom a Windowsom **rovnako ako objednávky**, vrátane
zlučovania, keď si na oboch počítačoch napísal niečo iné, aj mazania.

Nestačilo pridať tabuľku. Musela dostať **tri veci naraz**: vlastnú trvalú
identitu riadkov, značky po mazaní a zápis v zozname zlučovaných tabuliek.
Keby chýbala hoci jedna, celý prenos súboru by fungoval — takže by to vyzeralo
dobre — ale pri **zlúčení** by ticho zostala len lokálna kópia. Pri mieste,
kde si držíš všetko dôležité, by to bola tá najhoršia možná chyba.

---

## Ako sú uložené dáta (a prečo tak)

Hárok si drží zoznam stĺpcov, riadok si drží zoznam buniek — **spárované
poradím**. Klasickejšie riešenie by bola tretia tabuľka s jednotlivými bunkami,
ale to by malo zmysel iba vtedy, keby sa niekedy hľadalo „daj mi jeden stĺpec
cez všetky riadky". To sa tu nikdy nestane — je to zápisník, číta sa po
riadkoch.

**Poradie je preto to jediné, čo sa musí strážiť.** Preto sa stĺpce nikdy
nemenia tak, že by som poslal nový zoznam a appka hádala, čo sa kam presunulo.
Sú presne tri operácie — pridať, premenovať, zmazať — a tie dve, čo sa dotýkajú
riadkov, prepíšu **všetky riadky naraz v jednej transakcii**. Buď sa zmena
stane celá, alebo vôbec.

---

## Čo som overil

- **Migrácia 031 spustená na skutočnej SQLite**, postavenej zo všetkých 31
  migrácií: prejde čisto, identita riadkov sa priradí sama, mazanie hárka aj
  riadku zanechá značku, a zmazanie hárka odstráni jeho riadky.
- **Päť Rust testov** na to, čo sa dá pokaziť: krátky riadok sa doplní (nie
  odmietne), dlhý sa oreže, nečitateľné dáta sa prečítajú ako prázdne, zmazanie
  stĺpca vyberie správnu bunku z **každého** riadku, pridanie stĺpca pridá
  presne jednu prázdnu.
- **Zátvorky vo všetkých dotknutých súboroch** proti 2.49.2 — **bez posunu**;
  oba nové súbory sú vyvážené.
- **Každý import sa dá dohľadať** v exporte druhého súboru.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Rust ani Node tu nie sú, prvý reálny build je
CI) a ako ti to sadne pri reálnom používaní.

**Čo by som vedel dorobiť, keď to poskúšaš:** zoradenie podľa stĺpca, pripnutie
hárka hore, alebo prepojenie zápisku s konkrétnym eventom či objednávkou —
to posledné si teraz nechcel, ale dá sa dorobiť.

---

**Verzia:** 2.51.0 (9 miest v 7 súboroch).
**Migrácie:** **031_notes** (nová), ďalšia voľná je 032.
