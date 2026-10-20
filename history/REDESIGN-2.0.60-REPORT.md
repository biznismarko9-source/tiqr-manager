# TIQR Manager 2.0.60 — Orders Active/Paid opravené + nové tlačidlo "Fix sync"

## Čo si mi napísal

*"hned som nasiel 2 veci na ktorych bude treba zapracovat, prva bude ta ze orders su zle pochopene, vidim
ze si urobil 2 okna active a paid, vsetko by malo byt v active, no presunut do paid by sa malo len to, co
ked rozkliknes a vo vnutri vidis pri tickets sold, tak to presunut do paid, inak vsetko ostava v active,
dalsia vec, ked urobis sale v dashboarde pred tabulkou, tak ked stlacis push sales, tak sa ten dany sale
nevyplni aj v tabulke, to treba opravit, nech sa dopisuje, taktiez dost velka vec, ktoru bude treba doplnit
je, ze ked importujem info z tabulky do dashboardu, tak onoto zabudne vyplnit Category pri orders (...)
hladame najlacnejsiu verziu, toto urobime samostatne, najskor vyries tie 2 chyby a az potom urobime tu
poslednu dolezitu"*

Presne ako si napísal: v tomto kole idú len prvé dve veci (Orders Active/Paid + Push sales). Kategórie
(3. bod) idú až v ďalšom kole, samostatne.

## Oprava 1: Orders Active/Paid bolo skutočne zle pochopené — teraz podľa predaných lístkov

V 2.0.59 som Active/Paid na Orders postavil na poli **Payment status** objednávky (teda či **ty** máš
zaplatené **svojmu dodávateľovi** za lístky) — presne to si označil ako zle pochopené. Opravené: teraz sa
to riadi tým, či sú **lístky v objednávke predané** — presne to isté pravidlo, aké appka už predtým
používala na Tickets (tam, kde rozklikneš objednávku a vidíš stav jednotlivých lístkov):

- **Active** — objednávka má ešte aspoň jeden lístok, čo sa dá predať (dostupný alebo vystavený).
- **Paid** — všetky lístky objednávky sú buď predané, alebo zrušené — nie je už čo predávať.

Payment status (zaplatil/nezaplatil si dodávateľovi) teraz na zaradenie do záložky **vôbec nemá vplyv** —
stále ho ale appka ukazuje ako svoj vlastný stĺpec v tabuľke, ako doteraz, len už nerozhoduje o záložke.

**Jedna vec, čo som si dovolil rozhodnúť sám a chcem to potvrdiť:** objednávka, kde sú **všetky** lístky
zrušené (nie predané), teraz padne do **Paid**, nie do Active — z rovnakého dôvodu, prečo to takto robí aj
Tickets (nie je už čo s ňou robiť, tak nemá zmysel, aby zavadzala v Active). Ak by si to chcel radšej
naopak (zrušená objednávka nech zostane v Active), napíš mi — je to jedna podmienka.

## Oprava 2: Push sales — presnú príčinu sa nepodarilo nájsť, tak som postavil robustnejšie tlačidlo

Toto bolo náročnejšie. Aby som problém nezačal opravovať naslepo v tvojom reálnom, produkčnom Google
Sheete, pýtal som sa ťa postupne na presné okolnosti a každú hypotézu, čo appka podľa kódu mohla mať, sme
si spolu vylúčili:

1. Objednávka už v hárku riadok **mala** (nešlo teda o chýbajúci riadok/značku).
2. Predal si **naraz všetky** lístky objednávky (nešlo o čiastočný predaj naprieč viacerými cenami).
3. Cieľové stĺpce (Site Listed/Payout/Status/...) boli **úplne prázdne** predtým, než si stlačil Push
   sales (nešlo teda o pravidlo "nedotýkaj sa riadku, kde už niečo je" — to je zámerná poistka appky, aby
   nikdy neprepísala niečo, čo si do hárku napísal ručne).
4. Všetky lístky mali **rovnakú cenu** (nešlo o to, že appka odmietne "jednu spoločnú hodnotu", keď sa
   lístky v objednávke líšia).

Podľa toho, ako je táto časť appky napísaná, by za týchto okolností Push sales **mal** bol zapísať — prečo
sa to u tejto konkrétnej objednávky nestalo, sa mi teda z dostupných informácií nepodarilo s istotou
uzavrieť. Namiesto ďalšieho dohadovania na tvojom živom hárku si navrhol praktickejšie riešenie: nové
tlačidlo, čo tieto veci medzi appkou a hárkom opraví priamo — presne to som spravil.

### Nové tlačidlo: "Fix sync"

Nastavenia → Integrácie → karta "Orders & Sales" → hneď vedľa "Push sales" pribudlo tretie tlačidlo,
**Fix sync**. Rozdiel oproti Push sales:

- **Push sales** zapíše hodnoty do hárku len vtedy, keď sú cieľové bunky **úplne prázdne** — ak je v
  ktorejkoľvek z nich čokoľvek, celý riadok nechá tak, ako je (zámerne, aby nikdy neprepísal niečo, čo si
  tam napísal ručne).
- **Fix sync** toto pravidlo **obchádza**: pozrie sa na každú bunku zvlášť a **prepíše len tie**, ktorých
  aktuálny obsah nesedí s tým, čo appka o danej objednávke skutočne vie. Bunku, čo už správnu hodnotu má,
  necháva úplne na pokoji — takže spustiť Fix sync opakovane, alebo na hárku, čo je už v poriadku, nič
  nepokazí ani nezmení.
- Aj Fix sync stále vyžaduje, aby mala objednávka **jednu jasnú, jednotnú cenu/predaj naprieč všetkými
  svojimi lístkami** (rovnaké pravidlo ako Push sales) — ak sa lístky v objednávke líšia, appka stále
  odmietne hádať, ktorú hodnotu do jedného riadku hárku dať, presne ako doteraz.
- Keďže toto tlačidlo **môže prepísať niečo, čo už v hárku je** (na rozdiel od každého iného tlačidla na
  tejto karte), appka si pred spustením vždy vypýta potvrdenie v samostatnom okne.

Použitie: keď vieš, že nejaký predaj (alebo prijatý pull) v appke sedí, ale do hárku sa napriek Push sales
nedostal, klikni na Fix sync namiesto ďalšieho dohadovania prečo.

## Ako som to overoval

```
cargo test --lib  -> 628 testov, všetky prešli (625 pôvodných + 3 nové pre Fix sync)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Tentoraz som nerobil čerstvý náhľad appky v prehliadači (na rozdiel od 2.0.59) — obe zmeny (nová podmienka
na Orders, nové tlačidlo na Settings) som postavil tak, aby do bodky kopírovali už existujúci, overený
vzor v appke (Orders používa presne tú istú komponentu ako Tickets, len inú podmienku; Fix sync je
štruktúrne to isté ako existujúce tlačidlo Push sales/Push orders, len s inými textami a vlastným
potvrdzovacím oknom) — riziko preklepu vo vzhľade je tu preto výrazne nižšie než pri niečom úplne novom.
Namiesto toho som pridal 3 nové automatizované testy priamo na novú logiku (Fix sync opraví len bunku, čo
naozaj nesedí; nechá na pokoji riadok, čo je už správne; aj naďalej odmietne hádať pri nejednotnej cene).
Ak by na obrazovke niečo nesedelo vizuálne, napíš mi a pozriem sa na to konkrétne.

## Čo teraz urobiť

1. Nainštaluj 2.0.60.
2. Skontroluj Orders — objednávky s ešte nepredanými lístkami by mali byť v Active, úplne vypredané/zrušené
   v Paid, bez ohľadu na to, či si ich už zaplatil dodávateľovi.
3. Skús na tej istej objednávke, čo ti Push sales nevyplnil, teraz **Fix sync** (Nastavenia → Integrácie →
   Orders & Sales) a skontroluj v hárku, že sa to doplnilo.
4. Daj vedieť, či ti sedí rozhodnutie o zrušenej objednávke (vyššie) — ak nie, oprava je rýchla.

## Zmenené súbory

**Frontend:**
- `src/pages/Tickets.tsx` — funkcia `inventoryStatus` (počíta Active/Sold out/Cancelled z počtu lístkov)
  teraz exportovaná, aby ju mohol použiť aj Orders
- `src/pages/Orders.tsx` — Active/Paid teraz podľa `inventoryStatus` (predané lístky), nie podľa Payment
  status
- `src/pages/Settings.tsx` — nové tlačidlo "Fix sync" na karte Orders & Sales (vlastný stav, potvrdzovacie
  okno, výsledkový blok — rovnaký vzor ako existujúce Push sales/Push orders)
- `src/lib/api.ts` — nová funkcia `forcePushSales`

**Backend:**
- `src-tauri/src/commands/orders_sheet_sync.rs` — pôvodná `apply_sales_push` teraz volá zdieľané jadro
  `apply_sales_push_internal` s prepínačom `force`; nová funkcia `force_push_sales_impl` a príkaz
  `force_push_sales`; 3 nové testy
- `src-tauri/src/lib.rs` — zaregistrovaný nový príkaz `force_push_sales`

**Verzia (8 miest):** `2.0.60`.

## STOP

2.0.60 hotové — Orders Active/Paid teraz sedí s tým, čo si popísal (podľa predaných lístkov, nie podľa
platby dodávateľovi), a pribudlo tlačidlo Fix sync, čo vie opraviť riadok v hárku, aj keď sa presná
príčina pôvodného zlyhania Push sales nepotvrdila. Keď toto potvrdíš ako funkčné, ideme na 3. bod
(automatické rozpoznávanie kategórie pri importe z hárku).
