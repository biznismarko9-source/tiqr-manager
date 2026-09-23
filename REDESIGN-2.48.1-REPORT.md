# TIQR Manager 2.48.1 — toto bola tá chyba

V 2.48.0 som ti napísal, že v logike syncu som čítaním chybu nenašiel, a že to
možno bude tým, že sťahovanie beží len pri štarte. **Bolo to horšie a bola to
skutočná chyba.** Našiel som ju.

---

## Čo bolo zle

Zlučovanie a sťahovanie smelo bežať **len pri jednom jedinom tiku** — tom
úplne prvom, hneď po otvorení appky.

A tu je ten problém: **v tej chvíli ešte zvyčajne nie je pripravené prihlásenie
do Googlu.** Takže ten jediný privilegovaný pokus dostal odpoveď „vypnuté" a
**minul sa, hoci nikdy žiadnu šancu nedostal.**

Každý ďalší tik — každých päť minút, celý deň — už právo zlučovať nemal. Len
ukázal pruh a čakal. Donekonečna.

Takže obe polovice tvojej sťažnosti majú jednu príčinu:

- **„stále sa nespájajú tie info"** — zlúčenie po štarte nemalo ako nastať.
- **„stále to nie je automatické"** — a keď si appku nechal otvorenú, už nikdy
  nedostalo druhú šancu.

To sedí aj s tým, čo si napísal: *„bolo tak že len si zapol appku a už
automaticky začalo robiť sync"*. Presne tak to malo fungovať — a ten jediný
pokus sa míňal naprázdno.

## Čo je teraz

**Zlučovať aj sťahovať smie každý tik.** Žiadne privilégium, žiadne okno
príležitosti, ktoré sa dá premárniť.

**Jediné, čo to odloží:** keď máš **otvorené okno alebo rozpísané políčko**.
Dôvod je konkrétny: zlúčenie do databázy iba **pridáva riadky**, takže samotné
dáta nie sú v ohrození — ale obnovenie stránky, ktoré po ňom príde, by ti
zmazalo rozrobenú objednávku. Vtedy ti to napíše do pruhu a **dokončí sa hneď,
ako dopíšeš**.

**Navyše sa syncuje aj vtedy, keď sa na okno vrátiš** (focus). Predtým si po
prepnutí z druhého počítača čakal aj päť minút. Toto je väčšina toho pocitu,
že to nie je automatické.

## Prečo sa to nezacyklí

Toto som overil zvlášť, lebo teraz to môže bežať oveľa častejšie.

Po zlúčení sa uloží **verzia z Drive** (takže „druhý počítač má novšie" prestane
platiť) a databáza zostane označená ako **špinavá** (lebo pribudli riadky).
Ďalší krok je preto **nahranie spojených dát hore**, a potom pokoj:

**zlúčiť → nahrať → ticho.** Žiadne kolo dookola.

---

## Čo som overil

- **Postupnosť zlúčenie → push → idle** prečítaná priamo v `cloud_merge.rs`:
  ukladá verziu aj čas, a `mark_local_clean` **zámerne nevolá**.
- **Zátvorky** v `Layout.tsx` proti 2.47.5 — **bez posunu**.
- **Po zmene nezostal mŕtvy kód** — starý prepínač „len pri štarte" je preč aj
  so všetkými odkazmi, strojovo overené.
- **Poistka na rozrobenú prácu** pokrýva otvorené okno, rozpísaný input,
  textarea, select aj editovateľný text.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad a samotné spojenie na Drive. Ale ak to aj
teraz zlyhá, **2.48.0 ti už povie presný dôvod** — červený pruh hore a história
posledných 8 pokusov v Settings → Data, zvlášť na každom počítači.

---

**Verzia:** 2.48.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
