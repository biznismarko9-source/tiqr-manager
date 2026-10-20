# TIQR Manager 2.77.0 — Inventory

**Dátum:** 10.10.2026
**Zadanie:** redesign sekcie Inventory

---

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/OrderDetail.tsx` | 12 ručne kreslených kariet → `StatCard`; 4 popisky panelu zjednotené; doplnené importy |
| `src/pages/Orders.tsx` | 6 filtrov do jednej vnorenej lišty |
| `src/components/ui.tsx` | `StatCard` dostal tón `warning` (aditívne) |

Žiadna iná sekcia sa nedotkla.

## 2. Hlavný nález

`OrderDetail` si **dvanásť** metrických kariet kreslil ručne cez `<Card>`
s vlastným popiskom a hodnotou, kým Dashboard aj Event Detail používajú
`StatCard`. To bol zdroj tej nekonzistencie.

Prevedené všetkých dvanásť — šesť v hlavičke objednávky (Quantity, Unit
price, Fees + other, Total cost, Paid, Outstanding) a šesť v Order summary
(Total tickets, Available, Sold, Purchase cost, Sales revenue, Realized
profit).

Jedna komponenta = jedny odsadenia, jedna typografia, jedno pravidlo
orezávania. Metrická karta teraz vyzerá rovnako na Dashboarde, v evente aj
v objednávke.

**Žiadna hodnota, formát ani podmienka sa nezmenila.**

## 3. „Partial" a nový tón

Paid a Outstanding ukazujú slovo **Partial** jantárovo, keď model objednávky
nemá čím podložiť skutočné číslo. `StatCard` jantárový tón nemal, tak som
pridal `tone="warning"`.

Je to **čisto aditívne**: žiadny existujúci volajúci ho neposiela, takže ani
jedna iná karta v apke sa nezmenila. `SummaryStat` má vlastnú trojhodnotovú
úniu a tú som nechal tak.

Overené: `tone` teraz pozná `default | positive | negative | warning`,
a všetkých **49** použití `StatCard`/`EmptyState` naprieč piatimi stránkami
posiela len platné hodnoty.

## 4. Akcenty podľa významu

Nie pre pestrosť — aby sa dalo jedným pohľadom oddeliť, čo je čo:

| | |
|---|---|
| violet | počty (Quantity, Total tickets, Available, Sold) |
| blue | vstupy na kus a tržba (Unit price, Fees + other, Sales revenue) |
| orange | náklad a dlh (Total cost, Purchase cost, Outstanding) |
| emerald | skutočne vybrané (Paid) |
| teal | zisk (Realized profit) |

Realized profit si cez `tone` drží presne to isté emerald/red/neutral
pravidlo, aké mal predtým — takže farba nemôže odbehnúť od tej, akú pre tú
istú vec používa Dashboard.

## 5. Zoznam objednávok

Šesť filtrov do jednej vnorenej lišty (`surface-sunken`), rovnako ako Events
v 2.76.0 — nech sa oba zoznamy čítajú rovnako. Tie isté vstupy, handlery,
šírky aj zalamovanie. Prepínač Active/Completed ostáva vpravo.

Tabuľky som sa nedotkol: všetky stĺpce, ich poradie aj význam sú nezmenené.

## 6. Čo som NEPRIDAL a prečo

**Súhrnný pruh nad tabuľkou** z prvého inšpiračného obrázka („5 items |
$34,324.69 total cost | …"). Vyzerá dobre, ale znamenal by počítať nové
agregáty — a to je funkcia, nie vzhľad. Tvoje zadanie hovorí neduplikovať
informácie a nevymýšľať funkcionalitu.

Ak ho chceš, povedz — je to samostatná, dobre ohraničená vec.

## 7. Čo som overil

**Spustené:**

| kontrola | výsledok |
|---|---|
| Props každého `StatCard`/`EmptyState` oproti typu (5 stránok) | **49 použití, 0 chýb** |
| Hodnoty `accent` oproti `METRIC_ACCENTS` | 0 neplatných |
| Hodnoty `tone` oproti novej únii | 0 neplatných |
| Každá JSX komponenta v `Orders.tsx` / `OrderDetail.tsx` má import | 0 nevyriešených |
| Zátvorky v troch zmenených súboroch | rozdiel oproti zálohám 0 |
| Zvyšné ručne kreslené metrické karty | 0 (zvyšné 2 popisky sú mimo panelu, zámerne) |

**NESPUSTENÉ:**

- **Build, lint, type-check, testy.** Prekontroloval som prostredie minulý
  týždeň aj teraz pri validácii 2.76.0: node, npm, cargo **ani Homebrew**
  na tomto Macu nie sú.
- **Inventory som nevidel bežať.** Ani zoznam, ani detail objednávky.
- Active/Completed, filtre, triedenie, výber riadkov, New Order, Delete,
  Edit, finance akcie, Add pull info, interakcie s tabuľkou lístkov —
  **kódom nedotknuté, ale ani jedno otestované za behu.**

Screenshoty Inventory som nerobil. Pri Dashboarde som repliky stavať mohol,
lebo tam idú čísla; tu je to tabuľka s desiatkami riadkov a replika by ti
povedala viac o mojom mocku než o apke.

## 8. Zostávajúce otázky

1. Lištu filtrov ani prevedené karty som nevidel vykreslené. Dvanásť
   `StatCard`-ov v mriežke `summary-bar` (min. 144px na kartu) by sa malo
   zalomiť rozumne, ale je to odhad.
2. Karty sú teraz o niečo nižšie než tie ručné (`p-3` vs `p-4`), takže obe
   mriežky budú kompaktnejšie. To je zámer, nie chyba — ale uvidíš to až ty.

## 9. Ďalej

Zastavujem sa po Inventory. Na ďalšiu sekciu čakám na tvoje slovo.
