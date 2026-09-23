# TIQR Manager 2.48.0 — autosync konečne povie, čo robí

Prešiel som celú cestu autosyncu, od tlačidla po Google Drive. Píšem ti najprv,
čo som **našiel**, a potom, čo som **nenašiel** — lebo to druhé je pre teba
rovnako dôležité.

---

## Čo bolo naozaj pokazené

**Autosync prehĺtal každú chybu. Doslova.**

V kóde bol prázdny `catch {}` s poznámkou, že offline a odhlásený stav sú
normálne. To je pravda — lenže tie prichádzajú ako **rozhodnutie** z backendu
(„Off", „Offline"), nie ako chyba. Čokoľvek, čo tam naozaj spadlo — odmietnutý
upload, token, ktorý sa nedal obnoviť, zaseknutý zámok — **zmizlo bez stopy**.

Dôsledok: počítač, ktorému sa sync každých 5 minút nedaril, vyzeral **úplne
rovnako** ako počítač, ktorý nemá čo poslať. Na oboch stranách. Koľko chcelo dní.

To je presne ten stav, v ktorom si — „nefunguje a nikto nepovie prečo".

## Čo je v tomto builde

**1. Červený pruh hore**, keď autosync zlyhá, s **konkrétnou chybou**, nie
s vetou „niečo sa pokazilo".

**2. Každý pokus sa zapisuje** — aj ten, ktorý nemal čo robiť. V **Settings →
Data**, v karte Cloud sync, je nový blok **„Automatic sync on this computer"**:
posledných 8 pokusov s časom, rozhodnutím a dôvodom. Zvlášť na Macu a zvlášť
na Windows — lebo „ide mi autosync **tu**" je otázka o tomto počítači.

Ten existujúci riadok „Last synced" ti to povedať nevedel: počíta aj
synchronizácie, ktoré si spustil ručne. Takže mohol pokojne hlásiť „pred
hodinou", zatiaľ čo automatika medzitým zlyhala dvanásťkrát.

## Čo som NEnašiel — a to je dôležité

Prešiel som mechanické miesta, kde sa takéto veci lámu, a **všetky sú
v poriadku**:

| Kontrola | Výsledok |
|---|---|
| Sú príkazy zaregistrované v backende? | áno, všetky štyri |
| Sedia názvy medzi frontendom a Rustom? | áno, presne |
| Je rozhodovacia tabuľka správna? | áno, všetkých 7 vetiev |
| Nastavuje sa „mám neodoslané zmeny"? | áno, hák pri každom zápise do DB |
| Nemôže sa zaseknúť zámok „sync beží"? | nie, čistí sa cez `Drop` |

**Takže: v logike syncu som čítaním chybu nenašiel.** Nemám ako ho odtiaľto
spustiť — nemám Rust, nemám tvoj Google účet ani druhý počítač. Preto som
opravil presne to, čo mi bránilo (aj tebe) vidieť, čo sa deje. **Po tomto
builde ti to pri prvom zlyhaní povie presný dôvod** a ja ho viem opraviť adresne.

## Jedna vec, ktorá možno nie je chyba, ale zámer — a je na tvojom rozhodnutí

Autosync **nahráva** hocikedy. Ale **sťahuje len pri štarte appky.**

Keď je appka otvorená a druhý počítač medzitým niečo nahrá, **nestiahne to** —
len ti ukáže pruh. Dôvod je vážny: stiahnutie **nahradí databázu a reštartuje
appku**, zlúčenie **obnoví stránku**. Keby to spravila, kým píšeš objednávku,
prídeš o rozrobené.

**Ak máš v hlave, že necháš oba počítače otvorené a ony sa samy zrovnajú — tak
sa to nikdy nestane.** A toto môže byť presne to, čo voláš „nefunguje na oboch
stranách".

Nemenil som to sám, lebo je to kompromis, nie prehliadnutie. Ale viem to
zmeniť, a najbezpečnejšie takto: nechať bežať **zlučovanie** aj počas práce
(iba pridáva riadky, nič nemaže ani nenahrádza). Povedz a spravím to.

---

## Čo som overil

- **Zoznam súborov** oproti 2.47.5: **1 pribudol** (`lib/autoSyncLog.ts`),
  0 zmizlo, **2 zmenené**.
- **Zátvorky aj JSX značky** proti 2.47.5 — **bez posunu** v oboch zmenených
  súboroch. Settings.tsx má vlastnú starú nevyváženosť z môjho hrubého
  kontrolóra; dôležité je, že sa mojou zmenou **nezmenila ani o jednu**.
- **Zápis do localStorage nikdy nespadne** — je celý v `try/catch`, takže
  súkromné okno alebo plná kvóta nezhodia samotný sync.
- **Čo príde z localStorage, sa nikdy neverí** — čo nemá správny tvar, sa
  zahodí, nie vykreslí.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a samotné volanie na
Google Drive.

---

**Verzia:** 2.48.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
