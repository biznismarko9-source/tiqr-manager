# TIQR Manager 2.55.0 — Sheets: tmavý, voľný, s alertmi

Vybral si si **návrh 01 (Classic)** a napísal k nemu štyri veci. Tu sú všetky
štyri.

---

## 1. Tmavší, ale tak, aby sa v ňom dalo robiť

> *„urobil ho tmavsi ale tak vyvazeny aby sa v nom dalo pracovat"*

Mriežka ide podľa témy appky (svetlá/tmavá prepínačom), ale **tmavá je tá,
ktorú som ladil**:

- **rám** (písmená, čísla riadkov) je na najtmavšom podklade
- **bunky** sú o stupeň svetlejšie

Vďaka tomu to čítaš ako **rozsvietené bunky na tmavom ráme**, nie ako jednu
čiernu plochu, v ktorej nevieš, kde si. Vybraná bunka má modrý rám, jej písmeno
aj číslo riadku sa podfarbia — vždy vieš, kde stojíš.

---

## 2. Hlavička už nič nediktuje

> *„nechcem aby podla toho horneho stlpca si mohol zapisovat len co je v nom,
> chcem aby to bolo volne a vedel s tym pracovat"*

**Toto je najväčšia zmena.**

Predtým mal stĺpec meno a to meno určovalo, čo do neho patrí — bol to vlastne
formulár. Teraz:

- hore sú **písmená A, B, C**, ako v Sheets
- meno stĺpca je **nepovinný štítok** v druhom riadku hlavičky
- **môže byť prázdny** a väčšinou aj je

**Stĺpec je len pozícia.** Píšeš čo chceš a kam chceš. Prvý riadok si môžeš
použiť ako vlastnú hlavičku, ak chceš — alebo nie.

Nový hárok = **14 prázdnych stĺpcov bez mien**, prázdna mriežka, a začneš
písať. Štítok ktorémukoľvek stĺpcu dáš (alebo zmažeš) cez
**Format → Label for column**.

*Poznámka k tvojim starým hárkom: tie, čo už mená majú, si ich nechávajú —
zobrazia sa ako štítok pod písmenom. Nič sa nestratilo, len to už nie je
povinné.*

---

## 3. Alerty

> *„nejake alert by som si tam chcel nastavit casovo a tak"*

**Insert → Reminder…**, alebo zvonček vpravo hore.

Napíšeš **čo**, vyberieš **kedy** (dátum aj čas), prípadne pridáš poznámku.
A vieš to **pripnúť ku konkrétnej bunke** — tá potom má malý **oranžový
rožtek**, takže hneď vidíš, na ktorom riadku niečo visí.

Keď príde čas, dostaneš **systémové upozornenie** aj hlášku priamo v appke.
Zvonček hore ukazuje číslo, koľko ich je po termíne.

Alerty vidíš všetky naraz — pre tento hárok aj pre ostatné — odklikneš ich ako
hotové, alebo zmažeš.

### Toto ti musím povedať na rovinu

**Funguje to, kým máš appku otvorenú.** Keď je zavretá, nič nevyskočí — čo
medzitým dobehlo, dostaneš pri najbližšom spustení.

Aby to fungovalo aj so zavretou appkou, musela by v pozadí bežať samostatná
služba. To je iná robota a musel by si si ju vypýtať. Napísal som to aj priamo
do toho okna, nech ťa to niekedy neprekvapí.

*Čas je uložený tak, ako ho napíšeš — 9:00 znamená deviatu tam, kde si, na
oboch počítačoch. Žiadne prepočty medzi zónami, ktoré by ti alert spustili o
hodinu skôr.*

---

## 4. Menu hore a prepínanie dole

> *„toto co je hore ze file edit data to je good aj tie zakladne funkcie, aj to
> dole prepinanie"*

**Hore:** File · Edit · View · Insert · Format · Data

| Menu | Čo je v ňom |
|---|---|
| **File** | nový hárok, premenovať, zmazať |
| **Edit** | vyčistiť bunku, zmazať riadok, zmazať stĺpec |
| **View** | zoom 75 – 160 % |
| **Insert** | +10 / +50 riadkov, stĺpec, **dnešný dátum**, **alert** |
| **Format** | štítok stĺpca, posunúť stĺpec vľavo/vpravo |
| **Data** | zoradiť A→Z, Z→A, zrušiť triedenie |

Pod menu je lišta s **Today**, **+ Column**, filtrom riadkov a zoomom, a pod
ňou **riadok s hodnotou** (`B2  fx  ...`), takže vidíš celý obsah bunky, aj keď
sa do nej nezmestí.

**Dole:** taby hárkov. Klikneš = prepneš, **+** = nový, **dvojklik** =
premenovať.

---

## Riadky

Mriežka vždy ukazuje **aspoň 40 riadkov**, aj keď je hárok úplne prázdny — máš
kam písať bez klikania.

Keď napíšeš napríklad do riadku 30, vznikne aj **všetkých 29 nad ním**, naraz.
Inak by sa ti to, čo si napísal do tridsiatky, uložilo ako prvý riadok a
poskočilo hore. Presne toto som si overil.

**Klávesnica** funguje ako predtým: šípky, Enter (dole), Tab (doprava), Esc,
Delete, Home/End, PageUp/PageDown, a písanie rovno prepíše bunku.

---

## Čo som nechytal

Synchronizáciu, Dashboard, Events, Inventory, Sales, Pulls, Finance,
prihlásenie, nastavenia, tému, navigáciu.

Databáza sa len **rozšírila** — jedna nová tabuľka na alerty (migrácia 033).
Nič existujúce sa nemaže ani neprepisuje, tvoje hárky sú tie isté.

Alerty sa **synchronizujú** medzi počítačmi ako všetko ostatné, aj s tým, že
čo už raz vyskočilo na Macu, nevyskočí znova na PC.

---

## Čo tam NIE JE — aby si to nehľadal

- **Kopírovanie a vkladanie medzi bunkami** (ani do Excelu)
- **Označenie viacerých buniek naraz**
- **Vzorce** (`=SUM(...)`)
- **Ťahanie stĺpcov myšou** a **menenie ich šírky**
- **Späť (Ctrl+Z)**
- **Opakujúce sa alerty** (každý pondelok a pod.) — zatiaľ jednorazové
- **Alerty so zavretou appkou** (vyššie)

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 33 migrácií** prejde na čistej databáze; alert dostane uid, mazanie
  nechá stopu pre sync, a **alerty odídu s hárkom**, keď hárok zmažeš
- **ktoré alerty vyskočia**: po termíne áno; budúce nie; už raz zobrazené nie;
  odklikané nie
- **posun času alert znova nabije**, rovnaký čas ho nechá ticho
- **odkliknutie „hotovo" umlčí** aj alert, ktorý je po termíne
- **`clean_when`**: 3 platné tvary prijaté, 8 neplatných odmietnutých, vrátane
  takého, na ktorom by staršia verzia spadla
- **vznik riadkov**: zápis do nakresleného riadku 30 vytvorí presne 30 riadkov,
  hodnota ostane na tridsiatom, pozície 0–39 bez dier, druhé volanie nepridá nič
- **všetkých 44 SQL príkazov** pustených proti skutočnej schéme
- **26 príkazov Rust↔appka 1:1**, 210 volaní registrovaných
- kontrola na zvyšky po premenovaní — čistá
- zátvorky a importy vo všetkých zmenených súboroch

**Neoverené:** stále nemám Node ani Rust, takže **appku nezostavím** — to
spraví tvoj build. Rust som ale prešiel ručne (minule som sa naučil, že sa
nikdy nekompiloval).

Keby čokoľvek nesadlo — hlavne farby v tmavom režime — pošli screenshot a
doladím to.
