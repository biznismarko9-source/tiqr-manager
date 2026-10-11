# TIQR Manager 2.83.0 — Notes

**Dátum:** 11.10.2026
**Zadanie:** kompletný redesign sekcie Notes

---

## 0. Prostredie

node, npm, cargo ani Homebrew tu nie sú. **Žiadny build, type-check, lint,
testy ani spustenie.**

## 1. Audit — a dvakrát ma opravil

Bod 1 zadania hovorí: *"Do not assume a feature is unused simply because it
is not immediately visible. Trace its implementation."* Držal som sa toho a
dvakrát ma to zachránilo pred nezmyslom.

### Nie je to contentEditable editor

Zadanie hovorí o „rich-text editore" a „formatting toolbar". V skutočnosti
tam nie je ani jeden `execCommand`, ani jedno `dangerouslySetInnerHTML`.

Je to **blokový editor**: `NoteBlock[]`, jedna `textarea` na blok, typy
`text` / `bullet` / `num` / `check` / `image` / `rule`, s veľkosťou
(h1/h2/p/small), šiestimi farbami, bold, italic a polovičnou šírkou pre dva
bloky vedľa seba.

To mení všetko — formátovanie je blokové, nie inline HTML. Keby som
k tomu pristupoval ako k rich-text editoru, robil by som nezmysly.

### Takmer všetko, čo zadanie navrhuje, už existuje

Bod 4 navrhuje osem vecí. **Sedem už tam bolo:**

| navrhované | stav |
|---|---|
| Pripínanie poznámok | **už je** — tlačidlo, položka v menu, aj `ORDER BY pinned DESC` v backende |
| Štítky | dáta áno, **zobrazenie nie** ← jediná medzera |
| Lepšie náhľady | **už je** — `preview` v riadku zoznamu |
| Rýchle vytvorenie | **už je** — tlačidlo + šablóny |
| Duplikácia | **už je** — `DuplicateDialog` |
| Kontextové akcie | **už je** — `Menu.tsx` |
| Dátum poslednej úpravy | **už je** |
| Navigácia medzi poznámkami | **už je** — podstránky ako spodné záložky |

Plus archív, drag-and-drop preradenie, vyhľadávanie naprieč podstránkami,
počet podstránok a obrázkov, odkazy na objednávky a predaje.

### Raz som sa pomýlil a opravil sa

Najprv som napísal, že `pinned` sa nezobrazuje. **Bola to chyba** — prečítal
som len časť toho zhluku tlačidiel vpravo hore v riadku. Pripínacie tlačidlo
tam je, aj s jantárovou farbou pri pripnutej poznámke. Overil som to znova,
než som čokoľvek zmenil.

## 2. Čo som naozaj zmenil

| súbor | čo |
|---|---|
| `src/pages/Notes.tsx` | štítok v riadku zoznamu; `↑`/`↓` na ikony; import |

Jeden súbor.

### Štítok

`Note.tag` je v type, backend ho vyberá v `SELECT`, editor ho zapisuje pri
`onBlur` — a `Notes.tsx` ho **nerenderoval ani raz**. Vystopoval som to:
slovo `tag` sa v tom súbore nevyskytovalo vôbec.

Čiže si mohol napísať štítok a potom zmizol — v zozname, vo vyhľadávaní,
všade.

Teraz je prvý v riadku metadát, ako tónovaný čip. Prvý zámerne: to je to, na
čo štítok je — rozoznať dve poznámky na prvý pohľad, skôr než prečítaš dátum
alebo počet podkariet.

**Dôležité: zmenu zatiaľ neuvidíš.** Máš jednu poznámku („OASIS CODES")
a žiadny štítok. Overil som to v databáze. Dátová cesta bola mŕtva a už nie
je, ale prejaví sa až keď nejaký štítok napíšeš.

### Šípky preradenia

`↑` a `↓` boli holé textové znaky — vykresľovali sa v akejkoľvek veľkosti
a hrúbke, akú písmo zvolilo, a sedeli inak než každý iný ovládač v tom
riadku. Teraz `IconChevronUp` / `IconChevronDown`, rovnaké ako všade inde.

## 3. Čoho som sa NEDOTKOL — a prečo

**Editora.** Bod 7 hovorí, že stabilita je priorita, a menuje presne tie
riziká: strata obsahu pri prepínaní poznámok, resetovanie kurzora,
poškodenie uloženia.

`Editor.tsx` má 1 295 riadkov, vlastný `draft` ref, `dirtyRef`, `saveRef`,
`fieldRefs` na každý blok a autosave. Prerábať to bez kompilátora kvôli
vzhľadu je presne ten obchod, ktorý sa neoplatí — riskoval by som tvoje
poznámky za krajší panel.

**Slovenčinu v UI.** „Poznámky", „Hľadať v poznámkach…", „Nová poznámka".
Zvyšok apky je po anglicky, takže je to nekonzistencia — ale je to **obsah,
nie štýl**, a nemením ti jazyk bez opýtania. Ak to chceš zjednotiť, povedz.

## 4. Čo som overil

| kontrola | výsledok |
|---|---|
| Každá JSX komponenta v `Notes.tsx` má import alebo definíciu | 0 nevyriešených |
| Zátvorky | rozdiel oproti zálohe 0 |
| `tag` sa teraz renderuje | áno, 1 miesto |
| Holé textové šípky | 0 zostávajúcich |
| `pinned` naozaj funguje (overené, nie predpokladané) | tlačidlo + menu + triedenie v SQL |
| Tvoje dáta: koľko poznámok má štítok | **0** — preto to zatiaľ neuvidíš |

Jeden falošný poplach: kontrolór hlásil `Menu` ako nevyriešenú. Je to default
import v zmiešanom tvare (`import Menu, { menuAt }`), ktorý môj regex
nepokrýva. Overil som to.

## 5. NEOVERENÉ

- **Build, type-check, lint, testy** — nedá sa.
- **Notes som nevidel bežať.** Ani zoznam, ani editor.
- Vytváranie, úprava, prepínanie a ukladanie poznámok, formátovanie,
  checklisty, farby, oddeľovače, obrázky, vyhľadávanie, podstránky —
  **kódom nedotknuté, ale ani jedno otestované za behu.**

## 6. Čo tým nedodávam

Zadanie chce *"a complete Notes redesign, not merely a proposal or a
collection of small CSS adjustments"* a odkaz na Apple Notes / Notion.

**To som nedodal a nebudem predstierať, že áno.** Dodal som audit, ktorý
našiel jednu mŕtvu dátovú cestu, a dve opravy.

Dôvod je v bode 3: podstatná časť toho, čo by taký redesign znamenal, žije
v tom 1 295-riadkovom editore, a ten bez kompilátora prerábať nebudem. To,
čo sa dalo zlepšiť mimo neho — zoznam, metadáta, ikony — som zlepšil.

Ak chceš ísť do editora, poďme na to ako Finance: po etapách, jedna vec
naraz, a ideálne až keď budeš mať prostredie, kde to viem skompilovať
a spustiť.
