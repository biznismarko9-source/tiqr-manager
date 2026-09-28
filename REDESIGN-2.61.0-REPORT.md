# TIQR Manager 2.61.0 — opravy v poznámkach, Sheets preč

Poslal si zoznam deviatich vecí. Tu sú všetky, plus zmazanie Sheets.

---

## Priradenie

### Funguje po podkartách

> *„to priradenie musi fungovat osobitne v kazdej podkarte zvlast nie spolu"*

**Každá podkarta má svoje vlastné priradenia.** Poznámka „Oasis kódy" môže mať
v podkarte `ABC123` priradený jeden lístok a v `DEF456` iný.

*Čo si priradil v 2.58.0, sa presunulo na **prvú podkartu** tej poznámky.
Nenechal som to visieť „nikde" — to by znamenalo, že to nevidíš na žiadnej
podkarte a myslíš si, že sa to stratilo.*

### Inventár cez objednávku

> *„tam by to malo fungovat ako objednavka ze sa ukaze cela order a ked ju
> rozkliknes vies si vybrat ci vsetky listky alebo len niektore"*

Presne tak. Vyberieš typ **Inventár** → ukážu sa **objednávky** → rozklikneš
jednu → a buď dáš **„Všetky (4)"**, alebo si **odškrtáš konkrétne sedadlá** a
dáš „Priradiť označené".

Rovný zoznam dvesto kódov lístkov nebol na výber, to bola len stena textu.

### Financie sa dajú prečítať

> *„ked k tomu nemas popis tak vidis bez popisu takze musi to byt nejak tak
> urobit aby sa v tom dalo pracovat"*

Záznam bez popisu už nie je riadok, čo nehovorí nič. Názov sa poskladá z toho,
čo ten záznam **naozaj má** — kategória, miesto, objednávka, účet — a vpravo je
**vždy suma a dátum**, so znamienkom `+` alebo `−`.

Keď nemá vôbec nič, napíše aspoň „Príjem" alebo „Výdavok".

---

## Podkarty

| | |
|---|---|
| **Nová podkarta** | pýta si názov **v okne appky**, nie v systémovom |
| **Premenovať** | dvojklik na podkartu |
| **Posúvať** | šípky `‹` `›` na aktívnej podkarte |
| **Zmazať** | `×` — a **pýta sa** |

---

## Písanie

### Nový riadok

> *„fungovanie s novym riadkom"*

**Enter ťa teraz hodí rovno do nového riadku.** Predtým sa riadok vytvoril, ale
kurzor ostal, kde bol, a musel si kliknúť. Rovnako aj mazanie prázdneho riadku
cez Backspace ťa vráti do predošlého.

### Posúvanie

> *„mala by byt moznost hybat s poznamkamy, textom, fotkami"*

Keď prejdeš myšou po riadku, vpravo sa objavia **↑ ↓** — posunieš text, nadpis,
checkbox aj **obrázok**. Poznámky v zozname tiež, a **ostanú tam, kde ich dáš**
(predtým poznámka odskočila hore vždy, keď si niečo upravil).

### Späť

> *„moznost vratit zmeny keby sa nieco pokazi alebo zle sa zapise"*

**Ctrl+Z** alebo tlačidlo **↶ Späť** hore. Drží **60 krokov dozadu**.

**Poviem to na rovinu, aby si sa na to nespoliehal viac, než treba:** je to
história **v rámci jednej otvorenej podkarty**. Keď prepneš podkartu alebo
poznámku, začína odznova. Nie je to verzovanie — je to záchrana toho, čo si
práve pokazil, čo je presne to, čo si pýtal.

---

## Bezpečnosť pri mazaní

> *„pri mazani dat este jedno okno naozaj chcete zmazat"*

**Pýta sa** pri mazaní poznámky, podkarty aj obrázka — a napíše, čo presne
zmizne a že sa to nedá vrátiť.

**Štítok aj dátum** majú teraz **krížik**, keď ich tam nechceš.

*V poznámkach už nie je ani jedno systémové okno — všetko sú okná appky.*

---

## Sheets preč

> *„to co tam pise stare harky kde su sheets tak to uplne zmazat"*

Celá sekcia hárkov je **preč z appky** — stránka, adresa, položka v paneli aj
ten odkaz „Staré hárky".

### Jednu vec som ale nespravil, a chcem, aby si o nej vedel

**Dáta z hárkov som nezmazal.** Tabuľky sú stále v databáze a stále sa
synchronizujú — len ich appka nikde neukazuje.

Zmazanie je **nevratné**. Ty si napísal, že má zmiznúť *to, čo tam píše* —
teda tá sekcia — a to je hotové. Zmazať aj to, čo si do hárkov kedysi napísal,
je iné rozhodnutie a to nechcem spraviť za teba.

**Ak chceš aj dáta preč, napíš a spravím to.** Dovtedy tam potichu ležia a
nikde neprekážajú.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **migrácia na naozajstnej 2.58.0 databáze**: staré priradenia sa presunuli na
  prvú podkartu, poradie poznámok ostalo presne také, aké bolo, a **zmazanie
  podkarty vezme jej priradenia, ale poznámku nechá**
- **prečíslovanie pri presúvaní**: skúšané na čistom poradí, na poradí s dierou
  po zmazanom riadku aj na duplicitách — vždy skončí `0..n-1` bez dier
- **všetkých 38 migrácií** na čistej databáze
- **40 SQL príkazov** v module poznámok proti skutočnej schéme
- **54 príkazov Rust↔appka 1:1**, 238 volaní registrovaných
- **každé pole a každé volanie** v novom výbere záznamov naozaj existuje
- poradie zlučovania: žiadna tabuľka sa nezlučuje skôr než tá, na ktorú ukazuje
- po zmazaní Sheets nezostal v appke **ani jeden odkaz** na tú sekciu

**Neoverené:** appku nezostavím (nemám Node ani Rust) — to spraví tvoj build.
