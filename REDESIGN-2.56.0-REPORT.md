# TIQR Manager 2.56.0 — farby, veľkosti, viac miesta

> *„tie cislovania riadkov a stlpocv budu miniaturne, ze si vies pridat
> taktiez viac stlpcov aj riadkov, vies si zvacsit zmensit riadok, jeho
> velkost, farba textu, proste nech je to viac komplxnejsie"*

Všetko z toho je vnútri.

---

## Číslovanie je maličké

Písmená stĺpcov (`A B C`) aj čísla riadkov sú teraz **výrazne menšie** a majú
**vlastnú veľkosť, nezávislú od buniek**.

To je dôležité: keď si riadky zväčšíš, číslovanie **ostane maličké**. Inak by
ti rástlo spolu s nimi a zaberalo miesto, ktoré chceš dať obsahu.

---

## Farba textu

Nad tabuľkou je nová lišta:

**Text** ◻ 🔴 🟠 🟢 🔵 🟣 ⚪ &nbsp;·&nbsp; **B** *I*

Klikneš na bunku, klikneš na farbu. **Druhý klik na tú istú farbu ju zruší.**
Prvá (sivá s priehľadnosťou) je „späť na normálnu". **B** a **I** sa prepínajú
nezávisle od farby — takže vieš mať červené tučné.

### Prečo sa ti farba nikdy neposunie

Toto som riešil najdlhšie a stojí za to vedieť prečo.

Farba sa neukladá „k stĺpcu B, riadku 4". Ukladá sa **v rovnakom tvare ako
samotné bunky**, takže keď presunieš stĺpec alebo zmažeš iný, **farba ide s tým
textom, ktorý si ofarbil** — nie s pozíciou.

Overil som to: ofarbil som kód načerveno, dátum nasivo, potom stĺpec presunul
dopredu, zmazal stredný a jeden pridal. Po všetkých troch operáciách bola
červená stále na tom kóde a sivá stále na tom dátume.

Keby som to uložil „na pozíciu", po prvom presune by ti farby preskákali na iné
bunky a nevšimol by si si to hneď.

---

## Veľkosti

| Čo | Kde |
|---|---|
| **Výška jedného riadku** | lišta: `Row − +`, alebo `reset` späť na východziu |
| **Šírka stĺpca** | lišta: `Col A − +` |
| **Výška všetkých riadkov** | Format → All rows 20 / 24 / 30 / 40 px |

Všetko má rozumné hranice (riadok 16–400 px, stĺpec 40–900 px), aby si si to
omylom nezmenšil na nulu.

Zoom (75–160 %) funguje navyše — zväčší všetko naraz.

---

## Viac riadkov a stĺpcov

- prázdnych riadkov je teraz **60** namiesto 40
- na lište: **+ 50 rows**, **+ column**
- v menu **Insert**: +10, +50, **+200 riadkov**, stĺpec, **+5 stĺpcov** naraz

---

## Jedna vec, ktorú ti appka povie

**Ofarbiť sa dá len riadok, do ktorého si už niečo napísal.** Riadok, ktorý je
len nakreslený (tie prázdne dole), ešte v databáze neexistuje, takže nemá čo
ofarbiť. Appka ti to napíše — nebude sa tváriť, že klik nič nespravil.

To isté platí pri zmene výšky jedného riadku.

---

## Čo sa nezmenilo

Tvoje hárky, riadky, text — nič. Migrácia **iba pridáva** štyri stĺpce s
východzími hodnotami. Čo si mal, vyzerá presne tak, ako vyzeralo, kým sám niečo
neofarbíš alebo nezmeníš veľkosť.

Synchronizácia: žiadna nová tabuľka, takže nič nové netreba zapájať — farby aj
veľkosti idú medzi počítačmi spolu s riadkami.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 34 migrácií** prejde na čistej databáze, a starý riadok bez formátov
  sa načíta presne ako predtým
- **farby aj šírky prežijú všetky tri operácie so stĺpcami** (presun → zmazanie
  → pridanie) a ostanú na svojich bunkách — to je ten hlavný test
- **riadok z pred-034 verzie** (bez formátov) sa preformátuje bez posunu
- **prepínanie farieb**: 10 prípadov; a **to, čo vyrobí appka, prejde Rustom
  nezmenené** — inak by ti farba po znovunačítaní preskočila späť
- **neznáme príznaky** sa zahodia, nie odmietnu (kvôli sync medzi verziami)
- **54 SQL príkazov** proti skutočnej schéme
- **30 príkazov Rust↔appka 1:1**, 214 volaní registrovaných
- zátvorky, importy, kontrola na zvyšky po premenovaní — čisté

**Neoverené:** appku nezostavím (nemám Node ani Rust) — to spraví tvoj build.
Rust som prešiel ručne. Keby farby v tmavom režime nesadli, pošli screenshot.
