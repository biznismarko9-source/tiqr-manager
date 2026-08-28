# TIQR Manager 2.0.70 — Dashboard "Show more", predvolený Paid, preč s "Other" pri mene

## Čo si napísal

*"tiez tuto cast mozes urobit dlhsiu, alebo daj tlacitko ku kazdemu see more a ukaze sa toho viac, pri new
order ten payment status davaj ako prve bez zmeny nech sa ukazuje paid, keby nahodou som nato zabudol, lebo
aj tak vacsinu casu to je uz paid, pri new sale toto modrym other odstranit"*

(plus štvrtá vec — registrácia s "pending approval" — na tú sa pýtam samostatne nižšie, keďže si vyžaduje
tvoje rozhodnutie skôr, než na nej začnem robiť.)

## 1. Dashboard — Recent events/orders/sales majú teraz "Show more"

Každá z troch kariet na Activity karte (Recent events, Recent orders, Recent sales) ukazuje ako predtým
prvých 5 záznamov, ale pribudlo tlačidlo **"Show N more"** pod zoznamom — klikneš a zobrazí sa až 15
najnovších (predtým appka na pozadí ani nesťahovala viac ako 5). Tlačidlo sa zmení na "Show less", ak ich
chceš znova schovať. Karta, ktorá má 5 alebo menej záznamov, tlačidlo vôbec nezobrazuje — nie je čo
rozbaľovať.

## 2. New Order — Payment status teraz predvolene "Paid"

Predtým sa nové Objednávky vždy začínali s "Unpaid". Keďže väčšinu času sú v momente zadávania už zaplatené,
pole teraz **predvolene ukazuje "Paid"** — ak naň zabudneš sadnúť, ostane správne nastavené. V rozbaľovacom
zozname je "Paid" teraz aj prvá možnosť (predtým Unpaid/Partial/Paid, teraz Paid/Partial/Unpaid). Pole je
samozrejme naďalej voľne prepínateľné, ak konkrétna objednávka ešte zaplatená nie je.

## 3. New Sale — preč s "Other..." pri mene predaja

Modré "Other..." tlačidlo vedľa "Sale currency" (umožňovalo ručne napísať hocijaký menový kód) je preč.
Bežný rozbaľovací zoznam ostáva presne taký, ako bol — vrátane toho, že ak lístky, ktoré predávaš, boli
kúpené v inej ako obvyklej mene (napr. AED), táto mena sa aj naďalej automaticky ponúkne a rovno predvyberie
v zozname. Zmizla len možnosť ručne napísať úplne inú, neočakávanú menu.

## Čo som overil

```
cargo test --lib   -> 693 testov, 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/dashboard.rs` — recent_orders/recent_sales/recent_events teraz sťahujú až 15
  záznamov namiesto 5 (samotné obmedzenie na 5 zobrazených zostáva na frontende).

**Frontend:**
- `src/pages/Dashboard.tsx` — nové "Show more/less" tlačidlo pre všetky 3 Recent karty.
- `src/pages/Orders.tsx` — Payment status v New Order defaultne "Paid", "Paid" prvé v zozname.
- `src/pages/Sales.tsx` — odstránený "Other..." prepínač pri Sale currency v New Sale.

**Verzia (8 miest):** `2.0.70`.

## Registrácia s "pending approval" — potrebujem od teba rozhodnutie

Toto som zámerne nechal na samostatnú otázku (uvidíš ju hneď po tomto súbore) — pozrel som appku a dnes tam
neexistuje žiadna databáza účtov ani "schvaľovanie", len samotné prihlásenie cez Firebase. Aby to fungovalo
naozaj (nielen ako nápis), treba pridať jedno miesto, kam sa dá pre každý účet uložiť "schválené/čaká", a
spôsobov, ako to urobiť, je viac — každý s iným pomerom "koľko práce teraz" vs. "aké pohodlné to bude pre
teba nabudúce". Radšej sa spýtam, než by som postavil niečo, čo by si musel nechať prerobiť.

## STOP — nič, čo by som potreboval spätne overiť

Šírky/rozloženie na Dashboarde som nemenil, len pridal tlačidlo pod existujúce zoznamy. Ak ti "Show N more"
na niektorej karte bude ukazovať iné číslo, než čakáš, over si počet — sťahuje sa presne 15 najnovších, nie
všetky historicky.
