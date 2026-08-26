# TIQR Manager 2.0.39 — Nová obrazovka počas sťahovania a inštalácie aktualizácie

## Čo je nové

Keď si v **Settings -> Software updates** klikne na "Download & install", appka teraz namiesto malého
riadku s progress barom v karte zobrazí celú novú obrazovku cez celé okno - TIQR logo, nápis "TIQR
Manager", percentá sťahovania, progress bar a animovaný loading indikátor, na pozadí s farbami z appkinej
vlastnej témy (modro-fialový prechod, mierne tmavší v dark mode). Funguje rovnako dobre na širokom aj na
najužšom okne. Priložil som aj dva screenshoty (svetlý aj tmavý režim), nech vidíš presne ako to vyzerá,
bez toho aby si musel čakať na skutočnú novú verziu.

Skontroloval som aj, že appka po dokončení inštalácie a reštarte naozaj pristane na Dashboarde (nie napr.
na Settings, kde si predtým klikal Download & install) - áno, funguje to automaticky, appka sa vždy po
reštarte spustí od začiatku na Dashboarde, netreba k tomu nič naviac programovať.

## Jedna vec, kde si nie som 100% istý, čo presne si myslel

Napísal si "namiesto windows" - to sa dá chápať dvoma spôsobmi a chcem byť k tebe úprimný, ktorý z nich
som spravil:

1. **Obrazovka v appke, kým sa aktualizácia sťahuje a inštaluje** (predtým malý riadok v Settings) - toto
   som spravil, je to plne v appke, viem to naprogramovať aj overiť.
2. **Samotné okno inštalátora**, čo sa krátko mihne TESNE po stiahnutí (kým appka reštartuje) - to je už
   mimo appky, technicky súčasť Windows inštalátora (NSIS), a keďže tu nemám žiadny Windows na testovanie,
   nemôžem to ani spraviť s istotou, ani si to overiť, že to naozaj funguje.

Urobil som (1) - a keď som si teraz prešiel aj svoj vlastný starší zoznam úloh, mal som tam už predtým
presne toto zapísané ("in-app update screen, potom pristáť na Dashboard"), takže si celkom verím, že je
to naozaj to, čo si myslel. Ak predsa len si myslel (2) - to krátke okno, čo sa mihne - daj vedieť, skúsim
to doladiť, len to bude iný typ zásahu (úprava vzhľadu samotného inštalátora, nie appky).

## Testy a build

```
cargo test --lib -> 501 passed, 0 failed, 3 ignored (backendu som sa vôbec nedotkol, len pre istotu)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Novú obrazovku som aj naozaj vyrenderoval (nie len teoreticky spočítal) - na najužšom aj najširšom okne,
vo svetlom aj tmavom režime, pri viacerých percentách sťahovania vrátane úplného začiatku (predtým než
príde prvé percento). Vo všetkých prípadoch: obrazovka presne vyplní celé okno (žiadny okraj ani medzera),
progress bar presne sedí na percentách, logo sa naozaj načíta.

## Zmenené súbory

**Frontend (2 súbory):** nový `src/components/UpdateOverlay.tsx` (samotná obrazovka), `src/pages/
Settings.tsx` (jedno miesto zmenené - kde sa predtým ukazoval riadok v karte, teraz sa zavolá táto nová
obrazovka; samotná logika sťahovania/inštalácie sa vôbec nemenila).

**Backend:** žiadna zmena.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.39`.

## STOP

1. Pozri si priložené screenshoty (svetlý aj tmavý režim) - je to presne to, čo si predstavoval?
2. Keď budeš mať nabudúce skutočnú aktualizáciu na stiahnutie, choď na Settings -> Software updates a
   sleduj, či sa objaví táto celoobrazovková obrazovka namiesto pôvodného malého riadku, a či appka po
   dokončení naozaj pristane na Dashboarde.
3. Ak si pod "namiesto windows" myslel to krátke okno inštalátora TESNE po stiahnutí (nie samotné
   sťahovanie) - napíš mi, doladím to, len to bude iný typ zmeny (viď vyššie).
