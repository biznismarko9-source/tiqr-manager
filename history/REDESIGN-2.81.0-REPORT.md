# TIQR Manager 2.81.0 — Finance Overview (etapa 1)

**Dátum:** 10.10.2026
**Zadanie:** pokračovať vo Finance redesigne, po etapách

---

## 0. Prostredie — hneď, ako si žiadal

Bod 2 zadania: *"If the environment lacks Node.js, package managers, Rust, or
other required tools, report this immediately."*

**node, npm, cargo ani Homebrew tu nie sú.** Prehľadal som to naposledy pri
validácii 2.76.0 — PATH, dvanásť bežných umiestnení, `find` cez `/usr/local`,
`/opt`, Xcode a `~/Library`, aj shell profily. Nič.

Takže: **žiadny build, type-check, lint, testy ani spustenie.** Bod 4 hovorí
*"Do not claim this criterion has been verified until you have actually
inspected both charts running in the application"* — nespustil som ich, teda
to netvrdím.

## 1. Etapa 1: Overview

Bod 11 dáva stupňovanie, ktoré som sám navrhol — Overview, Transactions,
Accounts, Reports, jedna po druhej. Toto je **Overview, celý**.

Bod 5D je zároveň férová kritika 2.80.0: *"Do not redesign only the top of
the page and leave the remaining content visually inconsistent."* Presne to
som vtedy spravil — graf a karty hore, zvyšok nie. Toto to dopĺňa.

## 2. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/finance/Overview.tsx` | farebné pruhy a percentá v rozpade kategórií; popisky a odznak prognózy |
| `src/components/FinanceCategoryBadge.tsx` | nový export `financeCategoryFill` (aditívny) |

## 3. Expenses by category

**Pruh nosí farbu svojej kategórie.** Predtým bol každý fialový, hoci
štvorček vedľa názvu mal vlastnú farbu — štvorček hovoril jedno, pruh druhé.
To je presne tá chyba, ktorú som v 2.75.0 opravil na karte platforiem.

Nová `financeCategoryFill` vracia **tú istú triedu**, akú používa štvorček,
nie druhú tabuľku farieb, ktorá by sa od nej mohla rozísť.

**Percentá** pribudli — zadanie ich menuje. Počítajú sa zo súčtu riadkov toho
zoznamu, nie z karty Expenses nad ním. Zoznam je to, čoho sú podielom;
cudzí menovateľ by spôsobil, že by na vlastnom zozname nesadli do 100 %.

## 4. Chyba, ktorú našlo pozeranie

Vykreslil som ten zoznam s **tvojimi vlastnými kategóriami**:

| | | |
|---|---|---|
| (uncategorised) | 6 996,75 € | 52 % |
| Biznis náklady | 6 371,58 € | 47 % |
| Predplatné | 72,54 € | 1 % |
| **Zdravie** | **25,00 €** | **0 %** |
| **Doprava** | **3,90 €** | **0 %** |

Tie dve posledné ukazovali **„0%"**, hoci o stĺpec vedľa stojí reálna suma.
To číta ako „nič sa tu neminulo", čo nie je pravda.

Riadok s nenulovou sumou, ktorý zaokrúhľuje pod jedno percento, teraz ukazuje
**„<1%"**.

Môj test pri tom hlásil aj orezaný text vo všetkých piatich názvoch — **to
bol falošný poplach.** Na screenshote boli všetky čitateľné; `scrollWidth`
je pri flex položkách s ellipsis nespoľahlivý. Píšem to, lebo bez pohľadu na
obrázok by som ti ohlásil problém, ktorý neexistuje. (Je to piaty taký prípad
v tejto session.)

## 5. Cashflow Forecast

Štyri popisky `<dt>` prešli na `.section-title`.

Odznak „Forecast" mal `bg-brand-50` a `uppercase tracking-wide`. Tá výplň je
presne tá sýta levanduľa, ktorú `StatCard` vo vlastnom komentári z 2.47.4
označil za chybnú v light mode — tvoje vlastné slová vtedy boli *"na light
mode je tam chybna farba pri tom profit widgete"*. A veľké písmená malé
popisky stratili v 3.0.

Teraz nesie tónovaný prstenec, rovnaký idiom ako `METRIC_ACCENTS`, ktorý je
overený na oboch podkladoch.

Zvyšok karty — štvorstĺpcový `<dl>`, farebné rozlíšenie, riadok s prognózou,
poznámka o nie-EUR dátach — som nechal. Bol už v poriadku.

## 6. Čo som overil

**Naozaj spustené:**

- Rozpad kategórií **vykreslený v prehliadači s tvojimi reálnymi dátami**.
  Percentá dávajú 100 %, pruhy majú výšku a farbu kategórie, nič sa nezalamuje.
- **Celá apka**, 28 stránok: **75 použití `StatCard`/`EmptyState`, 0 chýb** —
  žiadne neznáme props, žiadne neplatné akcenty ani tóny.
- Zátvorky v oboch zmenených súboroch: rozdiel oproti zálohám 0.
- `financeCategoryFill` a všetky ikony majú import.
- Zvyšné `bg-brand-50`: tri, a **ani jeden nie je chyba** — jeden je môj
  komentár, jeden je `hover:` stav v Accounts (krátky tint, nie pokojová
  výplň), jeden je v Dashboarde, ktorý je mimo rozsahu.

**NEOVERENÉ:**

- Build, type-check, lint, testy — nedá sa.
- **Finance som nevidel bežať.** Ani jeden zo štyroch podpohľadov.
- **Graf z 2.80.0 som stále nevidel vykreslený s účtovnými dátami.**
  Akceptačné kritérium z bodu 4 teda splnené **nie je** — je to doslova tá
  istá komponenta, ale potvrdiť to dvoma screenshotmi vedľa seba neviem.
- Dátové filtre, Personal/Business, vytváranie a mazanie transakcií a účtov —
  kódom nedotknuté, neotestované za behu.

## 7. Riziká

Nízke. Obe zmeny sú prezentačné: výmena triedy, pridaný `<span>`, a jeden
nový aditívny export. Žiadny výpočet, filter ani dátový tok sa nedotkol.

Jediné, čo si všimneš okrem farieb: pruhy sú teraz v `flex` kontajneri
s percentom vedľa, takže sú o tých ~36px kratšie.

## 8. Ďalej

Etapa 1 hotová. **Etapy 2–4 — Transactions, Accounts, Reports a dialógy —
čakajú na tvoje slovo.** Nezačínam ich, lebo tvoj bod 11 hovorí dokončiť a
overiť jednu etapu pred ďalšou.
