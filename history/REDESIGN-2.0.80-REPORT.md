# TIQR Manager 2.0.80 — Oprava Summary tabuľky v Google Sheete

> *"summary paid a unpaid nefunguju dobre v tabulke, az ked je payment status paid az vtedy sa to moze zapocitat do tej tabulky, inak nie"*

Pozrel som sa presne, ako je Summary/Summary-Paid/Summary-Unpaid tabuľka (v tvojom pripojenom Orders & Sales sheete, stĺpce AB-AG) počítaná, a našiel som dve reálne veci, ktoré s tým boli zle. Predtým, ako som čokoľvek menil, opýtal som sa ťa na obe (aby som ti neprepísal niečo iné, než si čakal) - potvrdil si mi presne to, čo je nižšie.

## 1. Total Revenue a Total Profit počítali aj nezaplatené predaje

Predtým: "Total Revenue" a "Total Profit" sčítavali **každý predaný lístok** - Paid aj Pending (ešte nezaplatené) dokopy. Jediné, čo skutočne rátalo iba Paid, bolo "Total Paid".

Teraz: "Total Revenue" aj "Total Profit" počítajú **iba** predaje, kde je Payout status doslova "Paid" - presne to isté pravidlo, aké už predtým malo "Total Paid". Keďže obe teraz počítajú to isté, "Total Revenue" bude odteraz vždy presne rovnaké ako "Total Paid" - to nie je chyba, je to presne to, čo si chcel.

"Total Unpaid" som nemenil - stále ukazuje, koľko je predané, ale ešte nezaplatené (potrebuje na to celkovú sumu zo všetkých predajov, nie len tú "Paid" časť).

## 2. Refund sa nikdy nedostal do sheetu - stará dáta tam zostávali navždy

Toto je skutočná chyba, ktorú som našiel pri hľadaní príčiny: keď v appke **refundneš** predaj, appka o tom sheet nikdy neinformovala. Site Listed/Payout/Status/Payout status/dátum predaja/paid-by pre ten riadok ostali navždy také, aké boli **pred** refundom - aj keby si spustil "Fix sync", ktorý inak vie opraviť skoro všetko.

Dôsledok: taký riadok sa donekonečna počítal do Total Revenue/Profit/Paid/Unpaid, hoci lístok už dávno nie je predaný. A ešte horšie - keby si niekedy znova spustil "Sales sync", appka by mohla tento starý riadok omylom vziať ako **nový** predaj a vytvoriť duplicitný sale s tými istými starými číslami.

Oprava: "Push sales" aj "Fix sync" teraz po refunde vymažú Site Listed/Payout/Status/Delivery status/Payout status/dátum predaja/paid-by pre ten riadok naspäť na prázdno - riadok vyzerá presne tak, akoby lístok ešte nebol predaný. Prestane sa počítať v Summary úplne, a keď ho neskôr naozaj predáš znova, ten istý riadok sa dá pokojne použiť nanovo. Pull/who pulled/how much pull stĺpce sa netýkajú - tie sa refundom nemenia (peniaze od pullera sú samostatná vec).

**Dôležité:** táto oprava sa spustí až vtedy, keď klikneš na "Push sales" alebo "Fix sync" (Settings → Integrations → Orders & Sales) - nie automaticky hneď pri kliknutí na Refund v appke (refund je okamžitá lokálna akcia, sheet sa aktualizuje len pri ďalšom syncu/push-i, presne tak ako doteraz fungovalo všetko ostatné v tejto sekcii). Ak si niečo nedávno refundol, odporúčam kliknúť na "Push sales" ešte predtým, než nabudúce spustíš "Sales sync" - inak riskuješ práve ten duplicitný predaj popísaný vyššie.

## Čo som overil

```
cargo test --lib   -> 747 testov, 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Medzi novými testami sú aj také, čo overujú presne toto: refundnutý predaj sa naozaj prestane počítať ako Paid aj ako Revenue/Profit; opakované spustenie "Push sales" na už vyprázdnenom riadku nič nerobí (bezpečné klikať znova); objednávka, kde je refundnutý len jeden z viacerých lístkov (ostatné sú stále reálne predané), sa nechá úplne na pokoji - presne tak, ako sa to dnes deje pri akejkoľvek inej "nejednotnej" objednávke.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/src/commands/orders_sheet_sync.rs` — Total Revenue/Profit formula gated na Payout status = Paid; nová funkcia `order_fully_refunded` + logika, ktorá po refunde vyprázdni starý riadok v "Push sales"/"Fix sync".

**Frontend:**
- `src/pages/Settings.tsx` — popisy tlačidiel "Push sales"/"Fix sync" doplnené o novú refund-clearing logiku.

**Verzia (9 miest v 7 súboroch):** `2.0.80`.
