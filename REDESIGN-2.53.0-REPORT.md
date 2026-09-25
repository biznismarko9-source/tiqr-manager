# TIQR Manager 2.53.0 — Notes

Prečítal si si 2.52.0 a napísal mi druhé, oveľa kratšie zadanie. Podstata:

> *„Create one clean place where I can write down and organize anything
> important I need to remember."*
> *„Apple Notes simplicity + Google Sheets structure."*
> *„Keep it extremely simple. Do NOT overengineer."*

A test, ktorý si mu sám dal: keď to otvoríš, máš si pomyslieť
**„I can just write something here."**

**Mal si pravdu — Workspace bol prekomplikovaný.** Tak som ho orezal.

---

## Najdôležitejšie najprv

**Neurobil som druhú sekciu.** Workspace som **premenoval a zjednodušil** na to,
čo si pýtal. Dve sekcie, ktoré obe robia poznámky a tabuľky, by boli presne tá
komplikovanosť, pred ktorou si ma varoval.

Takže: **Notes**, vlastná položka v paneli, adresa `/notes`, s Finance nemá nič
spoločné.

**Databázu som nemenil a žiadnu novú migráciu som nepridal.** Keby si v 2.52.0
niečo stihol napísať — aj record, aj task — je to tam, otvorí sa to ako
poznámka a **pri uložení sa to nezahodí**.

---

## Sú tam už len dve veci

`+ New` ti ponúkne presne dve možnosti:

| | Na čo |
|---|---|
| **Note** | normálne písanie — čokoľvek |
| **Table** | tabuľka so stĺpcami, ktoré si sám pomenuješ |

**Preč sú:** Records s vlastnými políčkami, Tasks so stavmi a termínmi,
kategórie, štatistické kartičky, Upcoming panel.

---

## Note

Veľké pole na písanie, nič viac. Nad ním lišta:

**H** · **B** · *I* · • · 1. · ☐ · Link

Klikneš a vloží to značku do textu. Tlačidlo **Preview** ti ukáže, ako to
vyzerá vysádzané — a **checkboxy sa tam dajú rovno odklikávať**, zapíše sa to
späť do textu.

### Prečo je to obyčajný text

Lebo si napísal *„Do not build a complicated text editor. The main goal is fast
writing."*

Takže vnútri to nie je žiadny editor s vlastným formátom — je to **obyčajný
textarea**. To, čo napíšeš, sa presne tak aj uloží. Keby som tú obrazovku o rok
zahodil, tvoje poznámky sú stále čitateľný text, nie niečo, čo treba
rozkódovať.

Značky sú tie, čo poznáš: `# nadpis`, `**tučné**`, `*kurzíva*`, `- odrážka`,
`1. číslovanie`, `[ ] úloha`, `[názov](odkaz)`. A `☐` / `☑` tiež fungujú, lebo
si ich tak sám napísal v zadaní.

### Ukladá sa samo

Chvíľu po tom, ako prestaneš písať. Keď klikneš preč. A ešte raz, keď odídeš
späť na zoznam. **Žiadne tlačidlo Uložiť** — nikdy nie je stav, že si niečo
napísal a nie je to uložené. Vpravo hore stále vidíš `Saved` / `Saving…` /
`Unsaved`.

Každá poznámka má **názov**, **dátum** (nepovinný), **tagy** a vieš si ju
**pripnúť** hore hviezdičkou.

---

## Table

Funguje ako doteraz — **každá bunka sa uloží sama**, keď z nej klikneš preč.
Pribudlo to, čo si pýtal:

- **Triedenie** — klikneš na názov stĺpca. Druhý klik otočí poradie, tretí ho
  zruší a tabuľka je späť vo svojom poradí. Prázdne bunky idú vždy dole — prázdna
  bunka nie je malá hodnota, je to chýbajúca hodnota.
- **Filtrovanie** — políčko nad tabuľkou. Píšeš a riadky sa zúžia na tie, čo to
  obsahujú. Vedľa vidíš `3 of 12 rows` a odkaz `clear`.
- **Presúvanie stĺpcov** — šípky `‹` `›` v hlavičke. Stĺpec sa posunie **aj s
  bunkami** vo všetkých riadkoch.

Triedenie a filtrovanie sú **len zobrazenie** — nič neprepisujú. Presun stĺpca
je naozajstná zmena a ide cez databázu, v jednej transakcii.

### Opravil som pritom aj chybu, ktorú som tam mal od 2.51.0

Keď si zmazal stĺpec **v strede** tabuľky, na obrazovke ostali staré hodnoty
pod novými hlavičkami. **V databáze to bolo správne** — len zobrazenie klamalo,
kým si neodišiel a nevrátil sa.

Bolo to tým, že bunky sú „neriadené" polia (preto sa dajú ukladať na blur) a
také pole si nové hodnoty pri prekreslení nevšimne. Už sa riadky po každej
zmene stĺpcov poriadne prekreslia. Týka sa to aj nového presúvania.

---

## Hľadanie

**Jedno políčko hore.** Hľadá naraz v:

- názvoch poznámok
- texte poznámok
- **názvoch tabuliek** (toto v 2.52.0 nebolo)
- bunkách tabuliek

Výsledky rozdelené na **Notes** a **Tables**. Pri poznámke vidíš kúsok textu
okolo toho, čo si hľadal.

**Tabuľka dostane jeden riadok, aj keď jej sedí dvadsať riadkov.** Zvyšok ti
len spočíta — `Code · 3 more rows`. Dvadsať riadkov, na ktorých je to isté meno
tabuľky, nie je nájdenie, to je zavalenie.

---

## Usporiadanie

**Pinned → Recent → Tables → All notes**

Plus:

- **Triedenie zoznamu**: Recently updated · Newest · Oldest · Alphabetical
- **Tagy** ako filter — klikneš na tag, vidíš len jeho

*(Sekcia „Recent" sa ukáže, až keď máš viac ako 5 poznámok. Pri menšom počte by
to bol ten istý zoznam dvakrát pod sebou.)*

---

## Archív

V zadaní si archív nespomenul, ale 2.52.0 ho mal. Keby som ho vyhodil, čokoľvek
archivované by sa stalo neviditeľným.

Tak som ho **nechal, ale schoval**: dole je odkaz `Show archived (2)`. Na karte
poznámky je malé `Archive` / `Restore`. Nič viac.

---

## Čo som nechytal

**Synchronizáciu.** Ani riadok. Žiadne nové spúšťače, nič prerobené.

Dashboard, Events, Inventory, Sales, Pulls, Finance, prihlásenie, nastavenia,
téma, navigácia — nič z toho som neupravoval.

**Databázu som nemenil.** Žiadna nová migrácia, žiadny nový stĺpec, žiadne
prepisovanie existujúcich dát.

---

## Čo tam NIE JE — aby si to nehľadal

- **Typy stĺpcov v tabuľkách** (číslo, dátum, áno/nie) — **každá bunka je stále
  obyčajný text**. Miesto na to je v databáze pripravené, ale nič to nečíta.
- **Klávesové skratky.**
- **Ťahanie stĺpcov myšou** — presúvajú sa dvoma šípkami, nie drag & drop.
- **Zložky / vnáranie poznámok.**
- **Kalendár** — dátum si k poznámke pripneš, ale žiadny kalendárový pohľad.

Ak ti niečo z toho bude v praxi naozaj chýbať, povedz a dorobím to.

---

## Čo som overil a čo nie

**Overené (spustené, nie prečítané):**

- všetkých 32 migrácií prejde na čistej databáze — nič som nerozbil
- **presúvanie stĺpcov** na naozajstnej databáze: stĺpec aj bunky idú spolu,
  tam-a-späť vráti presne pôvodný stav, a **krátky riadok** (taký, čo môže
  vzniknúť po zlúčení z druhého počítača) to prežije bez posunutia
- **checkboxy**: všetkých 5 zápisov (`[ ]`, `[x]`, `- [ ]`, `☐`, `☑`) sa správne
  prečíta a odkliknutie sa dá vrátiť späť bez straty textu
- **značky formátovania**: 8 prípadov, ani v jednom sa nestratí text
- **lišta nad písaním**: 6 prípadov vrátane prázdneho dokumentu a poslednej
  riadky bez konca riadku
- **triedenie a filtrovanie** tabuľky vrátane prázdnych buniek
- **hľadanie**: tabuľka nájdená podľa mena sa vypíše raz, aj keď jej sedia 4
  riadky
- všetkých 204 volaní medzi appkou a Rustom sedí 1:1
- zátvorky vo všetkých nových aj upravených súboroch sedia

**Neoverené:** nemám tu Node ani Rust, takže **appku nezostavím** — to spraví
až tvoj build. A neklikal som cez hotovú appku. Keby čokoľvek nesadlo, pošli
screenshot.

**Jednu chybu som našiel a opravil ešte u seba:** keď si v poznámke písal a dal
Späť, poznámka sa po doukladaní sama znova otvorila. Bolo to tým, že uloženie
„na odchode" prišlo až potom, ako sa editor zavrel.
