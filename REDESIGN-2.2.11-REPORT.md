# TIQR Manager 2.2.11 — Attention Center: 5 boxov + Dashboard cleanup

Poslal si mi jednu veľkú správu, rozdelenú na tri jasné časti (Attention Center, Dashboard
cleanup, Fulfillment Center) s pokynom, aby z toho boli **dva samostatné releasy**. Presne tak to
aj robím — tento report pokrýva len prvý release, **2.2.11**: ČASŤ A (Attention Center) a ČASŤ B
(Dashboard cleanup). ČASŤ C (Fulfillment Center) je druhý release, **2.2.12**, s vlastným
reportom, a posielam ho hneď potom, čo je hotový a otestovaný — presne ako si chcel ("dve jasne
oddelené časti").

Predtým, než som čokoľvek menil, som si prečítal `CURRENT_STATE.md` aj `PROTECTED_AREAS.md` a
pozrel som sa len na Dashboard/Attention Center/Sales/Sale Detail/delivery-payment status a
existujúcu navigáciu — žiadny full repo scan, presne podľa pokynu.

Nikde som sa ťa nepýtal na spresnenie. Pri dvoch miestach, kde zadanie nechávalo priestor na
výklad (čo presne znamená "počet položiek" na jednom boxe, a presná príčina scrollu na
Dashboarde), som urobil rozhodnutie sám a píšem ho tu nahlas nižšie, aby si to vedel opraviť, ak
si to myslel inak.

## ČASŤ A — Attention Center: 5 samostatných boxov namiesto jedného mixed feedu

Predtým appka ukazovala jeden spoločný zoznam, zoskupený podľa dôležitosti (Critical/Attention/
Info). Teraz je hlavným obsahom **5 vždy viditeľných boxov**, presne v tvojom poradí a s tvojím
pomenovaním:

**NO LISTING PRICE YET** · **NO ACTIVE LISTING** · **NOT DELIVERED YET** · **EVENT COMING SOON** ·
**MARKET ATTENTION**

Každý box má jasný názov, počet, krátky podtext popisujúci o čo ide, a je klikateľný. Klik na box
ho vyberie a **pod boxami** sa zobrazí zoznam presne tých tiketov/objednávok/eventov, čo do tejto
jednej kategórie patria (rovnaký riadkový vzhľad ako predtým — klik na riadok ťa aj naďalej
zavedie na existujúcu stránku objednávky alebo eventu, nič nové som tu nevymýšľal). Druhý klik na
ten istý box, alebo tlačidlo "Close" v detaile, ho zase zbalí. V jednej chvíli je vždy vybraná
najviac jedna kategória — už žiadny veľký mixed zoznam ako hlavný obsah, presne ako si žiadal.

Box, ktorý má práve 0 položiek, sa nezmizne (všetkých 5 je vidno vždy), ale nedá sa naň kliknúť —
nie je tam nič, na čo by sa dalo pozrieť.

**Odkiaľ presne pochádzajú dáta**: appka už dnes (od 2.2.8/2.2.9) posiela ku každej položke pole
"category" s presne piatimi hodnotami — to isté pole, čo sa doteraz nikde nezobrazovalo, len sa
podľa neho nič netriedilo. Táto úprava iba zmenila, PODĽA ČOHO appka zoskupuje ten istý zoznam,
čo appka aj tak už posielala (podľa kategórie namiesto podľa dôležitosti) — **žiadna zmena na
backende, žiadna nová logika, žiadne nové dáta.**

**Rozhodnutie, ktoré som spravil sám — čo presne znamená "počet položiek" na boxe.** Kategórie
"chýba cena"/"žiadny listing"/"cena mimo trhu"/"nedoručené" sú už od 2.2.9 zoskupené podľa
OBJEDNÁVKY — jedna objednávka s 40 nedoručenými tiketmi je vždy JEDEN riadok, nie 40. Číslo na
boxe počíta tieto RIADKY (teda objednávky/eventy), nie surový počet tiketov — rovnaká logika, akú
appka už predtým používala v starých nadpisoch skupín ("Critical (3)"). Keby si chcel na boxe
radšej vidieť "koľko tiketov" namiesto "koľko objednávok", je to jednoriadková zmena — daj vedieť.

**MARKET ATTENTION — všetky 4 tvoje pravidlá boli splnené už predtým, overil som to v kóde:**
- zobrazuje sa iba vtedy, keď pre daný event naozaj existujú Price Checker dáta — inak sa pre ten
  event vôbec nič nevytvorí (nie nulová hodnota, ale žiadny záznam);
- appka nikdy sama neurčuje ani nenavrhuje cenu — táto kategória len upozorní, že aktuálna cena
  tiketu (tá, čo si zadal ty) je výrazne mimo trhového priemeru, nič viac;
- section/row sa nikde v tejto časti kódu nečíta ako faktor pre cenu;
- tier/level sa nikde v tejto časti kódu nepoužíva na určenie ani zmenu ceny.

Toto všetko som si overil priamym čítaním backendového súboru (vrátane jeho vlastného testu, čo
presne toto tvrdenie kontroluje), nie odhadom — preto na tento box **nebola potrebná žiadna zmena
na backende**, len ho appka teraz ukazuje ako samostatný box namiesto riadku v mixed zozname.

**Čo som zámerne nechal bez zmeny**: ten druhý, starší "Attention" blok nižšie na tej istej
Activity záložke (štyri dlaždice Pulls/Pending sales/Missing listing price/Upcoming events, plus
zvonček hore vpravo) — to je iná, už dávnejšie hotová funkcia, nespomínal si ju, a jej úprava by
bola väčšia vizuálna zmena, než si žiadal.

## ČASŤ B — Dashboard cleanup: zbytočný scroll preč

Na Overview záložke som našiel jedno konkrétne miesto, čo mohlo spôsobovať zbytočný scroll celej
stránky: zoznam **"Sales by platform"** (dole pod grafom) nemal žiadny limit — rástol o jeden
riadok za každú ODLIŠNÚ platformu, cez ktorú si niekedy niečo predal. Keď máš platforiem viac
(tvoj screenshot ukazuje 4 — Seatiks/Viagogo/Discord/WhatsApp), tento zoznam sa dá donekonečna
predlžovať a s ním aj celá stránka.

Opravil som presne toto miesto: zoznam platforiem má teraz vlastný, interný scroll (posúva sa
sám v sebe, nie celá stránka) až od istej výšky — bežný počet platforiem (4-5) sa aj naďalej
zobrazí celý, bez akéhokoľvek scrollu. Popri tom som mierne zmenšil dva existujúce odstupy na tej
istej záložke (okolo grafu a nad kartami s číslami) — o jeden krok, nie žiadny redesign, len menej
prázdneho miesta.

Skontroloval som aj samotný scrollovací obal celej stránky (`Layout.tsx`) — ten je už správne
nastavený (scrolluje len vtedy, keď sa obsah naozaj nezmestí) a nič som tam meniť nemusel.

**Na zváženie**: v tomto prostredí neviem spustiť appku v reálnom prehliadači s presným
rozlíšením/mierkou tvojho monitora, takže presne to miesto, kde sa u teba scroll spúšťal, som
neoveroval naživo — vybral som si zoznam platforiem ako opravu, lebo je to jediné miesto na celej
Overview záložke, čo môže rásť bez akéhokoľvek limitu, takže je to správna oprava bez ohľadu na
to, čo presne u teba scroll spôsobilo. Ak by sa scroll aj po tejto verzii ešte objavoval, over si
prosím Windows "Display scaling" (100 %/125 %/150 %) — pri vyššej mierke sa do rovnakého monitora
zmestí menej skutočného obsahu, než ukazuje screenshot v rozlíšení 1920×1080.

## Zmenené súbory

**Frontend (jediná zmena kódu tejto verzie — žiadny backendový `.rs` súbor sa nemenil):**
- `src/pages/Dashboard.tsx` — Attention Center prerobený na 5 kategórií (nové
  `ATTENTION_CENTER_CATEGORIES`/`AttentionCategoryCard`, prepísaný `AttentionCenterBlock`,
  odstránené staré `ATTENTION_CENTER_GROUPS`/`AttentionCenterGroup`); `SalesByPlatformCard`
  dostala interný scroll; dva odstupy na Overview záložke o krok zmenšené.

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

## Čo som overil

```
cargo test --lib   -> 1006 passed, 0 failed, 3 ignored (presne rovnaké číslo ako pred touto
                       verziou - žiadny .rs súbor sa v 2.2.11 nemenil, takže toto len potvrdzuje
                       nulovú regresiu)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Manuálne som si prešiel logiku pre všetky kategórie (aj s reálne nulovým počtom, aj s viacerými
riadkami) a klik-a-zobraz správanie pre každý z piatich boxov — vrátane prípadu, keď posledný
riadok vybranej kategórie medzičasom zmizne (appka vtedy detail sama zavrie, namiesto toho, aby
ostal otvorený a prázdny).

---

Toľko k ČASTI A a B. Dve veci na tvoje rozhodnutie, keby si to chcel inak, než som odhadol: čo
presne znamená "počet položiek" na boxe (riadky/objednávky, nie surové tikety), a moje vysvetlenie
príčiny Dashboard scrollu (neoverené naživo v tvojom presnom rozlíšení). Všetko ostatné je presne
tak, ako si napísal. ČASŤ C (Fulfillment Center, 2.2.12) posielam samostatne, hneď ako bude
hotová a otestovaná.
