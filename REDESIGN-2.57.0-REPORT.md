# TIQR Manager 2.57.0 — ťahanie, spájanie, výplne, kalendár

Poslal si screenshot naozajstného Google Sheets a zoznam vecí. Prešiel som ho
bod po bode.

---

## 1. Veľkosti sa ťahajú myšou

> *„chcem aby sa dala menit velkost tych riadkov a stlpcov"*

Chytíš **pravý okraj písmena stĺpca** a ťaháš — šírka sa mení **živo**, vidíš
to počas ťahania. To isté pri **spodnom okraji čísla riadku** pre výšku.

Uloží sa to **až keď pustíš tlačidlo**. Keby som ukladal počas ťahania, jedno
potiahnutie by znamenalo stovky zápisov do databázy.

Tlačidlá `−` / `+` na lište ostávajú, keby si chcel presne po kúskoch.

---

## 2. Farby sú sýtejšie — a pribudla výplň

> *„farby nech su sytsie"*

Text je výraznejší v **oboch režimoch** (nie len v tmavom — ladil som obe).

**Nové:**
- **Výplň bunky** — 8 možností. Sú zámerne **oveľa svetlejšie** ako text, lebo
  text na nich musí ostať čitateľný.
- **Podčiarknutie** a **preškrtnutie**
- **Zarovnanie** vľavo / na stred / vpravo

Všetko sa kombinuje — vieš mať červený tučný text na žltej výplni zarovnaný
doprava.

---

## 3. Spájanie buniek

> *„nech sa daju spojit riadky do jedneho, aj policka"*

Na lište: **Merge: 2 · 3 · 4 · all · split**

- `2` / `3` / `4` spojí bunku s nasledujúcimi
- `all` spojí **celý riadok do jednej bunky** — presne to „spojiť riadky do
  jedného", hodí sa na nadpis nad tabuľkou
- `split` to rozdelí späť

**Text sa pri spojení nemaže.** Keď rozdelíš, dostaneš všetko naspäť.

A rovnako ako farby: keď stĺpec **presunieš alebo zmažeš**, spojenie ide s tým,
čo si spojil — nie s pozíciou.

*Spája sa vodorovne, v rámci riadku. Zvislé spájanie cez viac riadkov tam nie
je — povedz, ak ho potrebuješ.*

---

## 4. Kalendár a čas

> *„add calendar alebo time a da ti to policko"*

**Insert → Date… / Time… / Date and time…** — otvorí sa výber, klikneš a hodí
ti to do vybranej bunky. `Date…` je aj priamo na lište.

---

## 5. „Budeš vedieť čo a kedy sa deje"

Toto je druhá polovica a asi najužitočnejšia vec v tejto verzii.

Tlačidlo **Coming up**:

- prejde **celý hárok**, bunku po bunke
- nájde **všetko, čo vyzerá ako dátum** — `25/09/2026`, `2026-09-25`,
  `25.9.2026`, aj s časom za tým
- vypíše to **od najbližšieho**, a vedľa dátumu ukáže **čo je s ním v riadku**
- pripočíta aj tvoje **alerty**
- klikneš na riadok a **skočí ti na tú bunku**

Nehľadá to v nejakom „dátumovom stĺpci" — hľadá to **všade**, lebo stĺpce tu
nič nediktujú. Dnešné položky sú zvýraznené.

Overil som, že to **nepovažuje za dátum** kód ako `2026ABC`, cenu `180,00`,
`B2` ani `25/09` bez roku — inak by ti prehľad zaplnili náhodné bunky.

---

## 6. Ďalšie funkcie

> *„pridaj tam viac funkcii, nech to funguje top a nech je tam vsetko co vies
> pouzit, pohraj sa s tym viac"*

| | |
|---|---|
| **Vložiť riadok nad / pod** | Edit menu, presne na kurzore |
| **Duplikovať riadok** | aj s farbami, výplňami a spojeniami |
| **Vymazať formátovanie** | bunka späť do pôvodného stavu |
| **Zmraziť 1–3 riadky** | View menu — ostanú visieť hore pri rolovaní |
| **Súčet a počet** | dole vedľa hodnoty bunky: koľko je v stĺpci vyplnených a ich súčet |

Súčet rozumie aj tomu, ako píšeš ceny — `1 234,50` aj `180,00`.

---

## Čo som nechytal

Synchronizáciu, Dashboard, Events, Inventory, Sales, Pulls, Finance,
prihlásenie, nastavenia, tému, navigáciu. Migrácia **iba pridáva** dva stĺpce s
východzími hodnotami.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 35 migrácií** na čistej databáze
- **spájanie**: spojenie na 4, zmenšenie na 2, rozdelenie — a hlavne to, že
  zmenšenie **neostavia neviditeľné bunky**; spojenie za koncom riadku sa
  oreže; posledný stĺpec sa nedá spojiť
- **spojenie prežije presun stĺpca** a ostane na tých istých bunkách
- **farby**: 17 prípadov prepínania a to, že **appka a Rust vyrobia identický
  reťazec** — tu som našiel skutočnú chybu: appka pridávala štýly v poradí
  klikania, Rust ich triedil, takže `BUS` vs `BSU`. Vyzeralo to rovnako, ale
  reťazce sa rozchádzali. Opravené.
- **štyri abecedy príznakov sa neprekrývajú** (farba / výplň / štýl / zarovnanie)
- **rozoznávanie dátumov**: 10 tvarov prijatých, 12 nedátumov odmietnutých
- **65 SQL príkazov** proti skutočnej schéme
- **34 príkazov Rust↔appka 1:1**, 218 volaní registrovaných

**Neoverené:** appku nezostavím (nemám Node ani Rust) — to spraví tvoj build.
Ťahanie myšou sa mi tu naživo odklikať nedá, tak si ho prosím hneď vyskúšaj a
keby ťahalo divne, napíš.
