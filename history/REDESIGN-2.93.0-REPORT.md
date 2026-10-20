# TIQR Manager 2.93.0

## Najprv to, čo si mi vytkol

Mal si pravdu a nebudem to obhajovať. Predošlé kolá boli **vrstva tokenov** —
akcent, povrchy, linky. Kompozíciu obrazoviek som nechal tak, ako bola.

Toto kolo mení rozloženie.

---

## 1. Tá medzera v Events bola CHYBA

`colgroup` mal **8 stĺpcov, hlavička 7.**

V 2.76.0 som na tvoju žiadosť odstránil stĺpec **Stock** z hlavičky aj z tela
tabuľky — a jeho `<col>` som nechal. V oboch colgroupoch. Tabuľka preto
rezervovala **11,333 % svojej šírky pre stĺpec, ktorý nič nevykresľuje.**

To je presne ten prázdny pás na tvojom screenshote. Nebol to vzhľad, bol to
môj nedorobok spred siedmich vydaní.

**Prečo to tak dlho prežilo:** zablúdený `<col>` je pri čítaní kódu
neviditeľný. Nestojí nikde blízko `<th>`, ku ktorému patrí. Prejaví sa
jedine ako šírka, ktorá chýba.

Oba colgroupy sú teraz na 7 stĺpcoch, súčet **presne 100 %**. Šírky som
prerezal, nie len jeden zahodil:

| | pred | po |
|---|---|---|
| Event | 28 % | **30 %** |
| Date | 9 % | **12 %** |
| Status | 10 % | **11 %** |
| Tickets | 8 % | 8 % |
| Cost / Revenue / Profit | 11 / 11,33 / 11,33 % | **13 / 13 / 13 %** |
| *mŕtvy stĺpec* | *11,33 %* | **—** |

Dátum tým prestal byť „12.09.2…" a tri peňažné stĺpce sú rovnaké, čo drží
stĺpec čísel ako jeden stĺpec.

---

## 2. `.toolbar` — skutočný primitív, nie CSS záplata

Každá zoznamová obrazovka sa otvárala rovnako: **päť alebo šesť ovládačov,
každý s viditeľným popiskom nad sebou**, v zapustenej lište.

To je **formulár** — niečo, čo vypĺňaš zhora nadol. Zoznamová obrazovka nad
tabuľkou formulár nechce. Chce **toolbar**: jeden riadok, do ktorého siahneš,
keď chceš zmeniť jednu vec. Tie popisky stáli celý pás presne tam, kde má byť
tabuľka.

Popisky som **nezmazal** — sú `sr-only`. Ostávajú v DOM aj v poradí pre
čítačku obrazovky, len sa nekreslia. Každý ovládač aj tak nesie placeholder
alebo prvú voľbu „All …".

**Nasadené na Events, Inventory, Sales a Pulls (2×)** výmenou **jednej triedy
na obrazovku.** Markup vnútri ovládačov je nedotknutý — preto sa dali štyri
obrazovky spraviť naraz a bezpečne. To je aj dôvod, prečo je to primitív
a nie dvadsaťšesť ručných úprav.

**Events navyše preskupený:** prepínač stavu ide **prvý** — mení sa
najčastejšie a rozhoduje, na čo sa ostatné filtre vôbec vzťahujú. Hľadanie
berie zvyšok šírky. Ovládače majú skutočné `aria-label`.

---

## 3. Čo je stále štrukturálne pôvodné

Nebudem to obaľovať:

- **Dashboard** nad rámec riadku čísel z 2.90.0
- **Finance** Transactions / Accounts / Reports
- **Všetky dialógy a sekundárne obrazovky**

To je ďalšia reálna práca.

---

## 4. Overenie

Vyrenderoval som pred/po s **tvojimi vlastnými dátami**:

- mŕtvy pás vyšrafovaný načerveno verzus tabuľka dobiehajúca k okraju
- formulár so šiestimi popiskami verzus toolbar — tabuľka začína o celý pás vyššie

Plus vyváženie zátvoriek štyroch stránok aj `index.css`, obe triedy
komentárovej chyby — 0 nálezov.

**Nie je to kompilácia.** Node ani cargo na tomto Macu nie sú, CI je prvý
kompilátor, a z toho istého dôvodu sa nedá spustiť ani dev server.

---

## 5. Verzia

2.92.0 → **2.93.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
