# TIQR Manager 2.42.0 — nová objednávka je stránka a riadky

Presne tá kombinácia, ktorú si uložil v preview: **celá stránka**, tabuľkové
**riadky**, pridávanie tlačidlom, **event raz hore**, mena/pull/poznámka
viditeľné, bez pásu s rozdelením.

---

## Ako to teraz funguje

Vyberieš event a pridávaš riadky. **Jeden riadok = jedno miesto.**
Jedenásť polí, nič viac: ks · typ · sektor · rad · sedadlá · platforma ·
**cena za kus** · mena · pull · poznámka.

**Pull je gulička.** Zhasnutá znamená nie. Keď ju rozsvietiš, vedľa vyskočí
políčko na meno. Keď ju zase zhasneš, meno sa nestratí — o tom, či pull
existuje, rozhoduje gulička, nie to, či je v poli text.

**Dátum nákupu sa nepýta.** Opečiatkuje sa dnešným dátumom pri otvorení
stránky a ďalej sa nehýbe, aj keby si formulár nechal otvorený cez polnoc.
Prepísať sa dá na detaile objednávky.

## Čo nesedí, je vlastná objednávka

Dva riadky sa zlúčia do jednej objednávky **len vtedy**, keď sa zhoduje
všetko — sektor, rad, cena, mena, typ, platforma, pull — **a sedadlá idú
tesne za sebou**.

| Zadáš | Vznikne |
|---|---|
| 409/56 `23-24` · 102/25 `14-15` · 102/25 `19` | **3 objednávky** |
| 409/56 `23-24` · 102/25 `14-15` · 102/25 `16` | **2 objednávky** (druhá 14-16) |
| 102/25 `14-15` za 201 · 102/25 `16` za 180 | **2 objednávky** — iná cena |
| 102/25 `14-15` za `201,00` · 102/25 `16` za `201.00` | **1 objednávka** — tá istá cena, inak napísaná |

Tlačidlo dole rovno povie, koľko ich vznikne.

## Čo sa už nepýta — a kam to išlo

- **Poplatky a „ostatné náklady"** formulár nezbiera. Cena, ktorú napíšeš, je
  celý náklad na kus. Predtým to boli sumy za celú objednávku, ktoré appka
  rozpočítala — teraz nie je čo rozpočítavať.
- **Stav platby** zostáva `paid`, ako bol predvolený od 2.0.70.
- **Poplatok za pull** sa zadáva tam, kde vždy patril — na detaile objednávky.
  Gulička zapíše **kto** ťahal.

Ak niektorú z týchto troch chceš späť do riadku, je to jeden stĺpec — ale pri
poplatkoch sa musíme najskôr dohodnúť, či je cena, ktorú píšeš, s poplatkom
alebo bez neho. To je otázka o peniazoch, nie o formulári.

## Kam to vedie

`/orders/new`. Tlačidlá **New Order** v Inventory aj **New order for this
event** v detaile eventu vedú sem. Starý odkaz, ktorý nesie event v `state`,
sa presmeruje, takže nič nekončí naslepo.

**Starú modálku som nezmazal** — zostáva v súbore nepoužitá, jednu verziu, aby
si mohol porovnať. Povedz a zmizne.

---

## Čo som overil

Node ani Rust tu nie sú, takže preklad je až CI. Overené:

- **Pravidlo rozdelenia**, portované do Pythonu a prebehnuté na deviatich
  prípadoch: susediace sedadlá, medzera, duplicitné sedadlo, iná cena, iný
  pull, bez sedadiel (len počet), tri susediace riadky do jednej, a cena
  napísaná dvoma spôsobmi. **Všetkých deväť prešlo.**
- **Sedadlá vždy sedia s počtom kusov** v každej skupine — to je podmienka,
  ktorú backend vyžaduje (`insert_order_with_tickets`), inak by vytvorenie
  spadlo.
- **Vyváženosť značiek** v zmenených súboroch oproti balíku 2.41.0.
- **Žiadny chybný import** v novom súbore ani v tých zmenených.

**Čo overiť nedokážem:** ako to vyzerá a píše sa naživo. Hlavne si pozri, či
ti nechýba pás so zhrnutím „vznikne N objednávok" — v skladačke si vybral
*Neukazovať*, takže tam nie je; číslo je len na tlačidle. Je to jeden riadok
kódu vrátiť.

**Jedna vec, ktorú by som ti povedal aj keby si sa nepýtal:** vytvorenie troch
objednávok sú tri samostatné zápisy. Keď druhý zlyhá, prvý už existuje —
hláška ti povie, ktoré kódy vznikli. Nepredstieram vrátenie, ktoré databáza
neponúka.

---

## Čo ostáva z tvojho zadania

Rovnaký riadkový formulár pre **Sales** a **Pulls** („nieco taketo podobne daj
na vsetky"). Pulls zostávajú tam, kde sú — tak, ako si povedal.

---

**Verzia:** 2.42.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
