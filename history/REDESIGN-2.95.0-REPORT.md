# TIQR Manager 2.95.0 — tri odstránenia

## 1. Malé grafy pod KPI hodnotami — preč

**Druhýkrát.** A to je na tom to podstatné.

- **2.88.0** — odstránil som ich, lebo si povedal „tie male grafy odstran".
- **2.94.0** — vrátil som ich, lebo ich ukazovala tvoja referencia a brief
  k nej ich pýtal menovite („a restrained sparkline… where appropriate").
- **2.95.0** — pozrel si sa a povedal nie.

Tá druhá položka bola moja chyba. **Referencia je smer pre vzhľad
a kompozíciu — nie je to pokyn, ktorý prebíja niečo, na čo si už raz povedal
nie.**

Zapísal som si to natrvalo: do komentára v kóde, do `CURRENT_STATE.md`
a do pamäte. Sparkline sa do tých kariet vráti **len vtedy, keď si o ňu
povieš ty.** Ak to bude znovu pýtať nejaká referencia, poviem ti, že je to
v rozpore s tvojím rozhodnutím, a nechám to tak.

Komponent `Sparkline` a prop `spark` ostávajú v `ui.tsx` **nepoužité**,
presne ako medzi 2.88.0 a 2.94.0. Preto sa minule vrátili za pár minút —
a nechať ich tam nič nestojí.

---

## 2. „Profit & Loss" — preč z prepínača na Dashboarde

Prepínač teraz ponúka **Revenue · Cost · Sales**.

Odfiltroval som to **na mieste volania** (`DASHBOARD_METRICS`), nie vymazal
zo zdieľaného zoznamu `METRICS` — ten používa aj **Recap**, a ty si ukázal
na kartu Dashboardu, nie na Recap.

Profit na tej stránke **nezmizol** — ostáva ako vlastná KPI karta hore.

---

## 3. „Net" — preč z prepínača vo Finance

Prepínač teraz ponúka **Cashflow · Income · Expenses**.

Stojí za to povedať, prečo to dáva zmysel aj obsahovo: **Cashflow** je prvá
pilulka aj predvoľba a kreslí príjmy proti výdavkom na jednom grafe. To je
presne tá otázka, na ktorú Net odpovedal tretím spôsobom.

S ním sa zjednodušilo aj `headlineIsNet`. Bolo to
`chartMode === "cashflow" || metric === "profit"` — po odstránení pilulky sa
`metric` už nikdy nemôže rovnať `"profit"`, takže tá polovica podmienky bola
mŕtva. Nenechal som ju tam.

---

## 4. Overenie

Vyrenderované:

- KPI karty bez grafov — **nezostala po nich diera**, odsadenia sedia
- Dashboard: Revenue / Cost / Sales
- Finance: Cashflow / Income / Expenses

Plus vyváženie zátvoriek celého `src/` a obe triedy komentárovej chyby — 0.

Jednu vec som si pri tom opravil sám: konštantu `DASHBOARD_METRICS` som
najprv vložil **doprostred import bloku**. Beží to (importy sa hoistujú),
ale je to zlý tvar a `import/first` to vie zahlásiť — presunul som ju pod
importy.

**Nie je to kompilácia.** CI je prvý kompilátor.

---

## 5. Verzia

2.94.0 → **2.95.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
