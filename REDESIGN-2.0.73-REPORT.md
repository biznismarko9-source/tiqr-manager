# TIQR Manager 2.0.73 — Settings → Lookups, zjednodušené rovnako ako Integrations

## Čo si napísal

*"tak isto ako sa menili Integrations v nastaveniach ze sa zjednodusovali, to iste treba urobit s Lookups,
urobit to nejak viac simple, ale aby to stale splnalo svoj ucel"*

(Toto bola len prvá časť tvojej správy — zvyšok, 2FA/email verifikácia, push notifikácie, dashboard
upozornenia a animácie, riešim samostatne nižšie a v ďalších verziách. Pozri sekciu "Čo bude ďalej" úplne
dole.)

## Čo som zmenil

Rovnaký princíp ako pri Integrations v 2.0.65: **nič som nezmazal ani neobmedzil** — len som prestal
natrvalo zobrazovať text, ktorý si prečítaš raz a potom ho už nikdy nepotrebuješ vidieť znova.

**Predtým:** nad zoznamom platforiem bol vždy natrvalo zobrazený odstavec ("Purchase platforms show up when
recording an order; Selling platforms show up when recording a sale...") a rovnako nad Event categories
("Tag events (football, concert, etc.) to filter and color-code them..."). Zaberalo to miesto na obrazovke
navždy, aj keď to už dávno vieš naspamäť.

**Teraz:** pri nadpise "Platforms" a "Event categories" je malá ⓘ ikonka — podržíš nad ňou myš a presne ten
istý text sa ukáže ako bublinová nápoveda. Nič sa nestratilo, len to nie je natrvalo na očiach.

Samotné pridávanie/mazanie platforiem, prepínanie Purchase/Selling/Both, aj Event categories fungujú úplne
identicky ako doteraz — tam som nemenil nič, keďže to už bolo kompaktné (jedno políčko + tlačidlo Add +
zoznam) a nešlo o to, čo si označil ako neprehľadné.

## Čo som overil

```
cargo test --lib   -> 703 testov, 0 zlyhaní, 3 ignorované (nedotknuté - táto zmena je len frontend)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

## Zmenené súbory

**Frontend:**
- `src/components/icons.tsx` — nová `IconInfo` (ⓘ, rovnaký štýl ako ostatné ikony v appke).
- `src/pages/Settings.tsx` — nová malá `InfoHint(text)` komponenta (ikonka + natívna HTML bublinová
  nápoveda, žiadna nová knižnica); oba odstavce pri Platforms/Event categories nahradené touto ikonkou.

**Verzia (8 miest):** `2.0.73`.

## Čo bude ďalej

Z tvojej správy zostáva:

1. **2FA + email verifikácia pri registrácii** — napísal si "postupom časom", takže to beriem ako smer do
   budúcna, nie úlohu na teraz. Ozvi sa, keď na to bude čas.
2. **Animácie/efekty** — vybral si "menší, cielený krok" (jemné prechody na pár najviditeľnejších miestach).
   Robím to hneď po tomto — dostaneš to ako samostatnú verziu, aby si si to mohol pozrieť oddelene od tejto
   zmeny.
3. **Dashboard upozornenie vpravo hore + push notifikácie (desktop + email + Pushover)** — toto je najväčší
   kus práce z celej správy (nová logika na rozpoznávanie "dôležitých vecí", tri rôzne spôsoby doručenia,
   nastavenia s citlivými údajmi). Najprv si to poriadne premyslím (rovnako dôkladne ako pri oddelených
   dátach účtov v 2.0.72), než začnem písať kód — dostaneš to ako samostatnú, väčšiu dávku.

## STOP — nič, čo by som potreboval spätne overiť

Ide o čisto vizuálnu zmenu (text → tooltip) bez zásahu do žiadnej logiky — pokojne skontroluj, či ti bublinová
nápoveda dáva zmysel (podrž myš nad ⓘ vedľa "Platforms" a "Event categories" v Settings → Lookups), ale nič
tu nemôže pokaziť tvoje dáta.
