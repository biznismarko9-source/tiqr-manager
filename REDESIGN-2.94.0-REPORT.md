# TIQR Manager 2.94.0 — Deep Black × Electric Purple

## 1. Cashflow preč

Mód **Cashflow** je z karty grafu na Dashboarde odstránený. Karta ostala —
je to teraz čisto metrikový graf so štyrmi módmi.

Samotný komponent `CashflowChart` som **nezmazal**: vykresľuje ho aj
**Finance Overview** a tá v tvojom zadaní nebola. Zmazať ho by znamenalo
rozbiť obrazovku, o ktorej reč nebola.

---

## 2. Tvoje tokeny, doslova

| token | hodnota | kde žije |
|---|---|---|
| Background | **#08090D** | `slate-950` |
| Surface | **#11131B** | `--surface` |
| Elevated | **#171927** | `--surface-raised` |
| Border | **#272938** | `--line` + prstenec karty |
| Primary purple | **#8B5CF6** | `brand-500` |
| Soft lavender | **#C4B5FD** | `brand-300` |

Tvoje dve fialové sedia na 500 a 300 rampy, takže to **nie je môj výklad
tvojej farby — je to tvoja farba.**

**Tlačidlo sa vracia na bielu na `brand-600`.** V 2.92.0 som ho prehodil na
tmavý text, lebo jasná tyrkysová bielu pri AA neunesie. #7c3aed ju unesie
(5,9 : 1). Rozhoduje kontrastná hranica, nie vkus — a preto sa to otočilo
oboma smermi.

---

## 3. Sparkliny sú späť

V 2.88.0 si mi ich kázal odstrániť. Tento brief ich pýta doslova a
referencia ich ukazuje — takže sa vracajú. Nie je to protirečenie, je to
tvoje novšie rozhodnutie.

Dôležité: **sú skutočné.** Každá je stĺpec z `data.revenueTimeSeries` —
tie isté koše, ktoré kreslí graf pod nimi. **ROI žiadnu nemá**, lebo tá
séria per-bucket ROI neobsahuje, a vymyslená čiara je presne to, čo táto
appka nesmie nikdy robiť.

Komponent `Sparkline` som v 2.88.0 zámerne **nechal v kóde**, keď som ho
prestal používať. Preto sa dnes vrátil za pár minút a nie za pol dňa.

---

## 4. Tvary widgetov

| | pred | po |
|---|---|---|
| rádius panelu | 8 px | **12 px** |
| ikonová dlaždica | 32 px / r8 | **36 px / r10** |
| hodnota vedúcej karty | 24 px | **25 px** |
| ostatné hodnoty | 19 px | **20 px** |
| podlaha KPI pásu | 9 rem | **11 rem** |

Tá podlaha nie je kozmetika: päť kariet, ktoré teraz nesú ikonu, hodnotu,
trend **aj** sparklinu, sa pri 9 rem rozdrví skôr, než sa riadok vôbec
rozhodne zalomiť.

---

## 5. Farba

Vedúca KPI karta nesie **fialový gradient wash**, nie svetlejší prstenec.

Dôvod: prstenec v tejto appke znamená **„vybrané"**. Net profit nie je
vybraný — je to len číslo, na ktoré sa pozeráš prvé. Nádych to povie bez
toho, aby si požičal význam, ktorý akcent nesie všade inde.

---

## 6. Čo z referencie ešte NIE JE

Bez obaľovania:

- **trojstĺpcový operačný riadok** — Upcoming Events / Recent Activity /
  Ticket Inventory + Pending Sales tak, ako ich referencia radí (obsah
  v appke existuje, usporiadanie nie)
- **celošírková tabuľka Event Performance** na spodku
- **horný toolbar s globálnym hľadaním** — stále ho nepostavím ako atrapu

---

## 7. Overenie

Vyrenderoval som KPI pás s ikonami a sparklinami, fialový wash na vedúcej
karte, graf s fialovou plochou a donut platforiem — v tvojich presných
hodnotách.

Jednu vec mi render chytil skôr, než sa k tebe dostala: ten gradient wash
by bol **prekryl text karty** (`::before` s `absolute` sa kreslí nad
nepozicovaným obsahom). Opravené cez `isolate` + `-z-10`, takže nádych sedí
pod obsahom a neutečie z karty von.

Vyváženie zátvoriek celého `src/`, `index.css` 110/110,
`tailwind.config.js` 12/12, obe triedy komentárovej chyby 0.

**Nie je to kompilácia.** CI je prvý kompilátor.

---

## 8. Verzia

2.93.0 → **2.94.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
