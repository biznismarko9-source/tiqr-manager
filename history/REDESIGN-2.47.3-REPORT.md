# TIQR Manager 2.47.3 — celá appka je po anglicky

> „vsetko by malo byt v apke v anglictine, nie v ziadnom inom jazyku, pri tych
> vyplnovatkach je vsade slovensky"

Máš pravdu a je to moja stopa: vypĺňovače vznikli v 2.45.0 z toho slovenského
preview, ktoré si si vybral, a zostali slovenské. Vtedy som ti to napísal ako
otvorenú vec — teraz je prepnuté.

---

## Ako som to hľadal

Neprešiel som to očami. Napísal som sken, ktorý v **celom `src/`** najprv
odstráni komentáre a potom v každom reťazci a v každom texte medzi značkami
hľadá slovenskú diakritiku alebo slovenské slová.

Našiel **127 reťazcov v 8 súboroch**:

| Súbor | Reťazcov |
|---|---|
| OrderRowsModal | 36 |
| SaleRowsModal | 36 |
| PullRowsModal | 28 |
| EventRowsModal | 17 |
| ui.tsx (spodná lišta, ikonky) | 5 |
| AiImportPanel | 2 |
| Settings | 1 |
| finance/Transactions | 1 |

Po oprave som **ten istý sken pustil znova** — vrátil nulu.

## Čo sa zmenilo

Všetko, čo vidíš: nadpisy okien, hlavičky stĺpcov, tlačidlá, placeholdery,
chybové hlášky, potvrdenia aj popisky pre čítačky obrazovky.

Napríklad: *Nová objednávka → New order*, *Ďalšie miesto → Add seats*,
*Vytvoriť 2 objednávky → Create 2 orders*, *chýba cena za kus → price per
ticket is missing*, *Zrušiť → Cancel*.

**Slovenské trojtvary padli.** `lístok / lístky / lístkov` je v angličtine
`ticket / tickets` — takže na piatich miestach ubudla celá vetva navyše.
To isté pri objednávkach, eventoch, pulloch, predajoch aj „veci/vecí".

## Čo zostalo po slovensky — naschvál

**Komentáre v kóde.** Je ich tam dosť a citujú tvoje vlastné zadania — prečo je
niečo urobené tak, ako je. To je záznam rozhodnutí, nie text appky, a nikto ich
nevidí. Keby som ich preložil, stratil by sa presný zmysel toho, čo si napísal.

**Jeden regex vo `priceParse.ts`**, ktorý pozná `Kč`, `zł`, `лв`, `Ft` a `lei`.
To nie sú texty, ktoré appka zobrazuje — to sú značky mien, ktoré **rozpoznáva**
v nalepenej cene. Preložiť ich by znamenalo prestať ich rozpoznávať.

---

## Čo som overil

- **Sken pred a po**: 127 nálezov → **0**.
- **Zoznam súborov** oproti 2.47.2: **0 pribudlo, 0 zmizlo, 8 zmenených.**
- **Zátvorky aj JSX značky** proti 2.47.2 — **identické vo všetkých ôsmich**.
  To je tá dôležitá kontrola: pri prepisovaní viacriadkových blokov je
  najľahšie rozbiť práve značky.
- **Žiadny nepoužitý import.**
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú). A či ti niektoré
slovo nesedí — anglické názvy som volil tak, aby sedeli so zvyškom appky
(*Section*, *Row*, *Seats*, *Platform*, *Currency*, *Notes*), ale ak chceš
niečo inak (napr. *Qty* → *Quantity*), je to jedna zmena.

---

**Verzia:** 2.47.3 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
