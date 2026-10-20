# TIQR Manager 2.0.75 — Zvonček na Dashboarde

## Čo si vybral

Na otázku, čo presne by malo to upozornenie vpravo hore na Dashboarde ukazovať, si vybral **"Veci, čo appka už vie rozpoznať"** — teda žiadny nový systém na vyhodnocovanie dôležitosti, len prehľadné zhrnutie toho, čo appka aj tak už dávno sleduje a ukazuje v sekcii "Attention" nižšie na Dashboarde. Presne to je táto verzia.

## Čo som pridal

Vedľa prepínača Overview / Financials / Activity (vpravo hore na Dashboarde, presne kde si to chcel - *"v dashboarde hore vpravo"*) pribudol malý zvonček.

Zvonček sleduje tie isté 4 veci, čo aj sekcia "Attention" nižšie na stránke: nezaplatené platby, čakajúce predaje (peniaze od kupujúceho), tikety bez nastavenej predajnej ceny a blížiace sa eventy. Odznak na zvončeku ukazuje, koľko z týchto 4 vecí aktuálne nie je na nule - takže napríklad "2", ak máš nezaplatenú platbu aj blížiaci sa event, ale nič iné. Keď je všetko v poriadku, zvonček je úplne tichý - bez odznaku.

Farba odznaku je normálne jantárová (amber), rovnako ako všade inde v appke pri podobných upozorneniach. Zmení sa na červenú len vtedy, keď je najbližší blížiaci sa event už dnes alebo po termíne - presne ten istý prah, aký appka už dnes používa pri farbení jednotlivých eventov v zozname "Upcoming events".

Klikom na zvonček sa otvorí malé okienko s prehľadom - pri každej položke je počet a odkaz priamo na správnu stránku (Orders, Sales, Inventory), pri blížiacich sa eventoch klik prepne Dashboard rovno na záložku Activity, kde je celý zoznam.

Dôležité: zvonček **nevymýšľa nič nové** - berie presne tie isté čísla, čo appka už dnes počíta a posiela do sekcie "Attention". Nemôže sa teda nikdy stať, že by zvonček ukazoval niečo iné, než čo vidíš v tej sekcii nižšie - sú to doslova tie isté dáta, len na dvoch miestach.

Nič z tohto sa nedotklo backendu (žiadna zmena v Ruste) ani žiadnej inej časti appky - je to čisto nová súčiastka na Dashboarde.

## Čo som overil

```
cargo test --lib   -> 703 testov, 0 zlyhaní, 3 ignorované (nedotknuté - žiadna zmena v Ruste)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

## Zmenené súbory

**Frontend:**
- `src/components/icons.tsx` — nová ikonka zvončeka.
- `src/pages/Dashboard.tsx` — nová súčiastka `AlertBell` (odznak + okienko po kliku), zaradená vedľa existujúceho prepínača záložiek.

**Verzia (9 miest v 7 súboroch):** `2.0.75`.

## Čo bude ďalej

Ostáva posledný a najväčší kus z tvojej správy - **skutočné odosielanie upozornení** (desktop, email, Pushover do mobilu), aby si sa nemusel pozerať priamo do appky, aby si niečo dôležité zbadal. Tento zvonček na Dashboarde je základ pre to, čo sa bude posielať von - rovnaké 4 veci, len teraz aj mimo appky. Idem na to hneď ako druhú, samostatnú dávku (2.0.76) - je to väčší zásah (nové heslá/kľúče na uloženie, nová tabuľka, pravidelná kontrola na pozadí), takže si to poriadne premyslené naprogramujem a otestujem a pošlem ti to zvlášť.

## STOP — nič, čo by som potreboval spätne overiť

Čisto vizuálna a frontendová zmena, nič sa nemenilo v tom, ako appka počíta dáta. Skús kliknúť na ten nový zvonček vpravo hore na Dashboarde - ak momentálne nemáš nič, čo by appka označila za "attention", bude tichý (bez čísla); ak niečo máš, over si, že číslo na odznaku aj obsah okienka sedia s tým, čo vidíš v sekcii "Attention" nižšie na tej istej stránke.
