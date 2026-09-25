# TIQR Manager 2.54.0 — Sheets

> *„naozaj by som radsej urobil to ako realne google sheets uplne jednoduche"*

Spýtal som sa ťa na jednu vec — či „len tabuľky", alebo „tabuľky + poznámky".
Vybral si si **len tabuľky**. Tak je to presne tak.

**Sekcia sa volá `Sheets`.** Poznámky (voľné písanie, Preview, checkboxy, tagy,
pripínanie) sú **preč**.

Viem, že je to tretí názov za tri verzie — Workspace, Notes, Sheets. Ale zakaždým
sa zmenil obsah a názov má hovoriť, čo to je. Teraz sú to hárky, tak sa to volá
Sheets.

---

## Hlavná zmena: tabuľka sa konečne správa ako tabuľka

Doteraz bolo **v každej bunke políčko na písanie**. Fungovalo to, ukladalo sa
to, ale vyzeralo to ako formulár so stovkou okienok — nie ako Sheets. To si
mal na mysli.

Teraz:

| | Ako to je teraz |
|---|---|
| **Bunka** | obyčajný text. Rámček sa objaví **len v tej jednej**, v ktorej práve si. |
| **Výber** | modro orámovaná bunka, ako v Sheets. |
| **Šípky** | posúvajú výber po mriežke. |
| **Enter** | otvorí bunku; po napísaní ide **dole**. |
| **Tab** | ide **doprava** (Shift+Tab doľava). |
| **Esc** | zruší, čo si práve písal. |
| **Delete** | vymaže obsah bunky. |
| **Home / End** | skočí na začiatok / koniec riadku. |
| **Začneš písať** | bunka sa rovno prepíše — nemusíš nič otvárať. |
| **Čísla riadkov** | vľavo, 1, 2, 3… |
| **Mriežka** | čiary okolo každej bunky. |

### Prázdny riadok vždy čaká dole

**Nie je tam žiadne tlačidlo „Add row".** Dole je vždy jeden prázdny riadok —
napíšeš doň a riadok vznikne, a pod ním sa objaví ďalší prázdny. Presne ako v
Sheets.

*(Kým máš zapnutý filter, prázdny riadok sa schová — inak by ti zmizol hneď,
ako doň napíšeš niečo, čo filtru nesedí.)*

### Ukladá sa to samo

Po každej bunke, hneď. Žiadne tlačidlo Uložiť, nikdy nie je stav, že si niečo
napísal a nie je to uložené.

---

## Čo ostalo

- **Triedenie** — klik na názov stĺpca, druhý klik otočí, tretí zruší
- **Filtrovanie** — políčko nad tabuľkou
- **Stĺpce** — pridať, premenovať, zmazať, posunúť doľava/doprava
- **Mazanie riadkov** — krížik vpravo (objaví sa, keď prejdeš myšou po riadku)
- **Hľadanie** — jedno políčko hore, cez **všetky hárky naraz**, názvy aj bunky
- **Zoznam hárkov** vľavo, hárok vpravo — všetko na jednej obrazovke

---

## Poznámky z 2.52 / 2.53

Ak si si do nich niečo stihol napísať, appka ti hore ukáže žltý pásik s
ponukou **„Import as a sheet"**. Klikneš a spraví z nich hárok
**Notes from the old version** so stĺpcami Title / Text / Date / Tags.

**Nič sa nemaže** — originály zostávajú v databáze, takže keby sa ti import
nepáčil, nič nie je stratené.

Ak si tam nič nemal, **žiadny pásik sa neukáže** a stránka je presne taká
jednoduchá, ako si chcel.

---

## Čo som nechytal

Synchronizáciu — ani riadok. Dashboard, Events, Inventory, Sales, Pulls,
Finance, prihlásenie, nastavenia, tému, navigáciu.

**Databázu som nemenil, žiadna nová migrácia.** Tvoje hárky z 2.51.0 sú tie
isté hárky, tie isté riadky.

---

## Čo tam NIE JE — aby si to nehľadal

- **Kopírovanie a vkladanie medzi bunkami** (ani do/z Excelu) — schránka sa mi
  v tomto prostredí nedá overiť, tak som ju radšej nedal, než dal pokazenú.
  Ak ju chceš, povedz a dorobím ju.
- **Označenie viacerých buniek naraz** (ťahaním myšou).
- **Vzorce** (`=SUM(...)`).
- **Ťahanie stĺpcov myšou** — presúvajú sa dvoma šípkami.
- **Menenie šírky stĺpcov.**
- **Späť (Ctrl+Z).**
- **Typy stĺpcov** — každá bunka je stále obyčajný text.

---

## Čo som overil a čo nie

**Overené (spustené, nie prečítané):**

- **celé správanie mriežky** na modeli: písanie do prázdneho riadku, Enter,
  Tab, Escape, Delete, dorazenie na každý okraj, a to, že Escape ani Delete na
  prázdnom riadku **nevytvoria prázdny riadok v databáze**
- **úprava v zoradenej tabuľke zapíše do riadku, ktorý vidíš** — nie do toho,
  ktorý je na tom mieste uložený (to je presne ten druh chyby, čo ticho
  prehodí dáta)
- **našiel a opravil som pritom jednu vlastnú chybu**: Enter v prázdnom
  spodnom riadku nechal výber stáť na mieste namiesto toho, aby šiel na nový
  prázdny riadok pod ním
- všetkých 32 migrácií prejde na čistej databáze
- zátvorky vo všetkých nových aj upravených súboroch sedia, importy sedia

**Neoverené:** nemám tu Node ani Rust, takže **appku nezostavím** — to spraví
až tvoj build. A neklikal som cez hotovú appku. Keby čokoľvek nesadlo, pošli
screenshot a opravím to.
