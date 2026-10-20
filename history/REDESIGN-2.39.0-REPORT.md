# TIQR Manager 2.39.0 — Orders a Inventory sú jedna obrazovka, Margin zaniká

---

## 1. Orders + Inventory = jedna položka, volá sa Inventory

Keď si sa pýtal, ako by sme ich vedeli spojiť, odpoveď bola lepšia, než sa
zdalo: **už to bola tá istá vec.** Obe stránky volali `api.listOrders`, obe
vykresľovali objednávky (nie jednotlivé lístky) a obe viedli na ten istý
detail `/orders/:id`. Líšila sa len sada stĺpcov.

Takže spojenie nebola prerábka, ale odstránenie duplicity:

- V menu je **jedna položka — Inventory**.
- Prežil **`/orders`** (je to bohatšia stránka — New Order, úpravy, hromadné
  akcie) a dostal názov Inventory. Stĺpce sú **nezmenené**, presne ako si
  povedal.
- **`/tickets` presmeruje na `/orders` aj s query stringom.** To je dôležité:
  z detailu eventu sa odovzdáva `?code=<kód lístka>` a Orders to nájde, lebo
  jeho vyhľadávanie matchuje aj kódy lístkov (nie len kódy objednávok — je na
  to test v `orders.rs`).

**Prepojené odkazy:** Dashboard ×2, detail eventu ×2, prehliadka, a spätný
odkaz z detailu objednávky — ten už nevetví, vždy je „Back to inventory".

**Prehliadka mala dva kroky na `/tickets`.** Po spojení by dvakrát za sebou
ukazovala tú istú obrazovku, takže sa jeden zlúčil do kroku o objednávkach
(veta o listing price sa presunula doň). Zostáva **11 krokov** a Guide si ten
počet číta sám, netreba ho nikde prepisovať.

**Čo som nezmazal:** `pages/Tickets.tsx`. Detail predaja a detail objednávky
z neho importujú zoznamy stavov (`DELIVERY_STATUS_OPTIONS`,
`RESALE_STATUS_OPTIONS`) a modal na úpravu lístka. Odišla len jeho route a
položka v menu.

## 2. Margin zaniká

Odišla odvšadiaľ, kde ju bolo vidieť:

| Kde | Čo zmizlo |
|---|---|
| Dashboard → Financials | karta **Margin** |
| Dashboard → Financials | podriadok pri **Profite** (teraz už len ROI) |
| Dashboard → porovnanie období | riadok **Margin** |
| Detail eventu | karta **Margin** |
| Detail predaja | karta **Margin** + jej výpočet |

Detail predaja má teraz **päť kariet namiesto šiestich** a mriežka sa zúžila
s nimi — nie je tam prázdne miesto.

Prepísal som aj **texty**, ktoré o marži hovorili — dva kroky prehliadky a
Guide v Settings. Hovoria o zisku, nie o marži, aby appka nevysvetľovala
číslo, ktoré už neukazuje.

**ROI zostáva.**

Jedna vec na rovinu: backend `margin` **ďalej počíta a posiela**, len to už
nikto nečíta. Vymazať to z `finance.rs` znamená siahnuť na zdieľaný finančný
modul, ktorý volá každá obrazovka, má na seba testy, a ja to tu neviem
preložiť. Úspora by bola nulová a riziko zbytočné. Na obrazovke marža
neexistuje, čo je to, čo si chcel.

(`components/Recap.tsx` maržu tiež obsahuje, ale ten je od 2.38.0 nedostupný —
nič ho neimportuje, takže sa na obrazovku nedostane.)

---

## 3. Čo som NEurobil — a prečo

Písal si: **„vo financial tabe by mali byt ukazane listky len tie co vlastnim
nie aj tie co som ich pullol."**

Pozrel som sa do databázy a vyzerá to inak, než sa zdá zo screenshotu:

**Pully, ktoré ťaháš pre niekoho iného, sa do inventára nemajú ako dostať.**
Tabuľka `pulls` nemá `order_id` ani žiadny odkaz na lístok — a jej vlastný
komentár v migrácii to hovorí priamo: cena lístka „je zaplatená kartou toho
druhého a **nie sú to markove peniaze ani jeho výdavok**". Ukladá sa len
**tvoja odmena** (`price_cents`).

Tie čísla na screenshote (35 available, 61 purchased, €5 055,27) sa počítajú
**výlučne z tabuľky `tickets`**, a tie vznikajú len z objednávok, ktoré si
vytvoril ty.

Takže jedna z dvoch vecí:

1. **Myslíš `pulls_received`** — lístky, ktoré niekto potiahol *pre teba*. Tie
   `order_id` majú, lebo si ich naozaj kúpil a vlastníš ich — a preto do
   inventára patria. Ak ich tam nechceš, je to zmena pravidla, nie oprava.
2. **Pully si zapisuješ ručne ako objednávky**, aby si ich mal kde sledovať.
   Potom ich treba vedieť **označiť** (príznak na objednávke) a z inventára
   ich odrátať — to je migrácia + zmena vo finančnom module.

Nešiel som do toho, lebo obe možnosti menia **chránenú finančnú logiku** a
jedna z nich aj databázu. Povedz mi, ktorá to je, a spravím to.

---

## 4. Čo som overil

Na tomto Macu nie je Node ani Rust, takže preklad je až CI. Overené:

- **Žiadny visiaci odkaz na maržu** — `header.margin`, `data.period.margin`,
  `s.margin`, `now.margin`/`prev.margin`: všetko na nule. Pomocné funkcie
  (`formatPercent`, `formatPercentOrMixed`, `computeTrendPoints`) sa všade
  ďalej používajú pre ROI, takže nezostal nepoužitý import.
- **Žiadny odkaz na `/tickets`** mimo samotného `Tickets.tsx` a presmerovania.
- **Vyváženosť značiek a zátvoriek** v deviatich zmenených súboroch oproti
  balíku 2.38.0 — žiadny nový nepár.
- **Päť kariet v päťstĺpcovej mriežke** v detaile predaja.
- **11 krokov prehliadky**, žiadny dvakrát za sebou na tej istej stránke.

**Čo overiť nedokážem:** ako to vyzerá naživo a či ti niektorý starý odkaz
nechýba. To uvidíš na prvom builde.

---

**Verzia:** 2.39.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
