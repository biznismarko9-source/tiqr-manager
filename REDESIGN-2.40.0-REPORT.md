# TIQR Manager 2.40.0 — tichšie stavy, čitateľnejší text, skeletony, návrat na riadok

Vybral si si desať vecí. Toto sú **štyri z nich, ktoré sú čistý frontend** —
žiadna migrácia, žiadny Rust, žiadny zásah do peňazí. Zvyšných šesť (Finance +
multi-sektor) ide ďalej, samostatne, aby sa dalo overiť, čo sa kde zmenilo.

---

## Najprv to, čo si musíš vedieť: dve z tvojich položiek boli už hotové

Neschovávam to do poznámok pod čiarou:

- **20 — prilepená hlavička tabuľky už v appke bola**, od verzie 2.6.0
  (`.table-shell thead th { position: sticky }`). Tá karta mala dve polovice a
  hotová bola tá prvá. **Druhú — zvýraznenie riadku po návrate — robí táto
  verzia.**
- **16 — skeletony v zoznamoch už boli**, tiež od 2.6.0. Koliesko „Loading…“
  zostávalo na **Dashboarde a vo všetkých štyroch kartách Finance**. Tie sú
  dorobené teraz.

---

## 17 · Badge je bodka a text

Stav prestal byť plná farebná pilulka. Zostala **farebná bodka a obyčajný
popis**. V tabuľke, kde má každý riadok stav, tie pilulky prekričali čísla
vedľa seba — a čísla sú to, prečo sa na tú tabuľku pozeráš.

Dôležité: **farby sa nemenili ani o odtieň.** `STATUS_TONES` je stále jediný
zoznam, z ktorého sa berie, ktorý stav má akú farbu. Pri vykreslení sa z neho
len vyhodí výplň a rámik. Žiadna druhá tabuľka farieb, ktorá by sa mohla
rozísť s prvou.

Bodka je o stupeň väčšia a v plnej sile — identitu teraz nesie sama.

**Ani jedno volanie sa nemenilo.** Všetkých 22 stavov som strojovo preveril, že
im po odobratí výplne zostáva farba textu pre svetlý aj tmavý režim, takže
žiadny stav nemôže vyjsť neviditeľný.

**Rozbaľovacie stavy na detaile predaja a objednávky si plnú pilulku nechávajú
zámerne** — to je ovládací prvok a musí vyzerať ako ovládací prvok.

## 18 · Kontrast — 260 miest

Toto bolo prehodené naopak: tmavší odtieň sivej sedel na tmavšom podklade.
Zmerané **3,16 : 1**, po oprave **4,16 : 1**.

Zámena prebehla na **260 miestach v 24 súboroch**. Predtým som overil, že ani
jeden výskyt nemá nalepenú predponu (`hover:`, `group-hover:` a podobne),
takže zámena nemohla chytiť nič iné, než mala. Po nej: starý pár **0×**,
správny **350×**.

Je to najväčšia zmena tejto verzie a zároveň tá, ktorú uvidíš všade — popisky
stĺpcov, podriadky pod číslami, poznámky pod kartami.

## 16 · Skeletony na Dashboarde a vo Finance

Namiesto kolieska nad prázdnom sa ukáže **tvar toho, čo príde**: riadok kariet
s číslami, potom panel grafu. Stránka si drží výšku a neposkočí, keď dáta
dorazia.

Pribudli dva tvary — `StatsSkeleton` a `PanelSkeleton`. V modáloch a pri
krátkych čakaniach vnútri panelov koliesko **zostáva**; tam je správne.

## 20 · Riadok, na ktorom si bol

Keď sa vrátiš z detailu, **riadok, z ktorého si odišiel, sa na chvíľu
rozsvieti** a zhasne. Žiadne tlačidlo, nič na zavretie — len odpoveď na „kde
som to bol“.

Funguje na **Events, Inventory a Sales**. V Pulls nie, tie vlastný detail
nemajú. Značka sa pri prečítaní spotrebuje, takže riadok blikne **raz po
návrate**, nie pri každej ďalšej návšteve zoznamu. Kto má v systéme vypnuté
animácie, nevidí nič.

---

## Čo som overil

Node ani Rust tu nie sú, takže preklad je až CI. Overené:

- **Všetkých 22 stavov + záložný** po odobratí výplne stále majú farbu textu
  pre svetlý aj tmavý režim (strojovo, portovaná logika do Pythonu).
- **Kontrastná zámena**: 260 výskytov, žiadny s nalepenou predponou, po zámene
  starý pár 0×.
- **Vyváženosť značiek a zátvoriek** v deviatich zmenených súboroch oproti
  balíku 2.39.0 — žiadny nový nepár.
- **Žiadny visiaci import** `LoadingBlock` tam, kde sa už nepoužíva, a žiadny
  chybný import v nových súboroch.

**Čo overiť nedokážem:** ako to vyzerá naživo. Hlavne dve veci si pozri sám —
či ti bodka namiesto pilulky nechýba v hustej tabuľke, a či blik riadku nie je
príliš silný alebo príliš slabý. Oboje je jednoriadková zmena.

---

## Čo ide ďalej

Zo zvyšku tvojho výberu:

- **24** transakcia priradená k eventu — potrebuje migráciu 031
- **26** opakované príjmy — tiež migrácia
- **29** porovnanie dvoch období — UI
- **30** oprava zostatku ako transakcia — **zásah do chránenej peňažnej
  logiky**, pôjde samostatne a s testami
- **25** import výpisu z banky — najväčší kus, pôjde sám
- **01** viac sektorov — odpoveď na štruktúru mám od teba: *čo nesedí a nie je
  vedľa seba = samostatná objednávka.* **Chýba mi druhá odpoveď:** zadávaš
  jednu cenu za celý nákup (a appka ju rozdelí medzi tie objednávky), alebo
  cenu za každú skupinu zvlášť?

---

**Verzia:** 2.40.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
