# TIQR Manager 2.0.44 — Login/Register, krok 1 z 2

## Čo si mi napísal

*"okej, mozme sa vrhnut na login/ register, potrebujeme urobit nejaku uvodnu stranku, tiqr, nejaka moznost
vyberu ci login alebo register, taktiez by bolo top ak by sa vedeli registrovat cez google, chcem to urobit
aj tak ze uvidim presne kedy sa niekto zaregistroval ako dev, chcem aby sa v ucte dole vlavo ukazal profil,
kde si vies kliknut a tam si budes vediet nastavit nejake zakladne info... tam bude aj moznost sa log
outnut, zatial podme nato pomaly, nic ine nemen, len toto pridavame"*

Vybral si si: **Google aj email+heslo**, a na uloženie účtov **Firebase**. Toto je krok 1 z 2 - obrazovky sú
hotové a dajú sa naozaj preklikať, ale zatiaľ na "nanečisto" prihlásenie (vysvetlené nižšie). Krok 2 (skutočné
Firebase prihlásenie) príde hneď, ako mi pošleš tie prihlasovacie údaje z Firebase konzoly.

## Čo je nové

**Úvodná obrazovka** - keď appku otvoríš, prvé, čo uvidíš, je TIQR logo a výber Log in / Sign up, s poľami na
email a heslo, aj tlačidlom "Continue with Google".

**Profil dole v sidebar** - presne tam, kde bol predtým len nápis "Local-first...", je teraz klikateľný
riadok s tvojím menom a emailom. Klikneš naň a otvorí sa menu s "Account settings" a "Log out". Ten pôvodný
nápis "Local-first..." tam ostal, len je teraz pod tým profilom.

**Nastavenia -> Account** - nová sekcia v Nastaveniach (popri Lookups/Data/Integrations/Appearance/Software),
kde si vieš zmeniť meno, vidíš svoj email a spôsob prihlásenia, a je tam aj tlačidlo Log out.

## Dôležité - toto je zatiaľ "nanečisto"

Prihlásenie/registrácia teraz FUNGUJE naklikateľne - vieš sa zaregistrovať, uvidíš svoje meno v profile,
odhlásiť sa - ale nie je to ešte napojené na Firebase. Zatiaľ appka len:
- príjme akýkoľvek email a heslo (nič sa nekontroluje, nie je čo kontrolovať)
- "Continue with Google" ťa prihlási pod vymyslenou identitou ("Google User")
- uloží si to len u teba v appke (lokálne), nikde inde

Toto je schválne - chcel si najprv vidieť a preklikať si obrazovky, než to napojím na niečo ozajstné. Dáta o
tvojich objednávkach/lístkoch/predajoch sa touto zmenou vôbec nedotýkajú - sú úplne oddelené a ostávajú presne
tak lokálne ako doteraz.

## Čo teraz urobiť

1. Otvor appku - mal by si vidieť novú úvodnú obrazovku namiesto rovno Dashboardu
2. Skús sa zaregistrovať (email+heslo, alebo cez Google tlačidlo) - preklikni si to
3. Skús kliknúť na svoj profil dole v sidebar, pozri si menu, choď do Account settings, skús zmeniť meno a
   uložiť, skús sa odhlásiť
4. Ak si mi ešte neposlal ten Firebase `firebaseConfig` z krokov, čo som ti dal v chate - pošli mi ho, keď
   budeš mať chvíľu, a spravím krok 2 (skutočné prihlasovanie)
5. Ak sa ti niečo na obrazovkách nepáči (farby, texty, poradie polí...) alebo chceš niečo inak, pokojne mi
   napíš - toto je presne ten správny čas na zmeny, kým to ešte nie je napojené na nič ozajstné

## Testy a build

```
npx tsc -b     -> 0 chýb
npm run build  -> OK
```

Táto zmena je len frontend (React/TS) - žiadny Rust súbor sa nemenil. Naviac som celý tok (registrácia,
profil, nastavenia, odhlásenie) prešiel automatizovane cez skutočný prehliadač (Playwright) so
screenshotmi na kontrolu - bez jedinej chyby v konzole.

## Zmenené súbory

**Nové (2):** `src/lib/auth.tsx`, `src/pages/Welcome.tsx`.

**Upravené (3):** `src/App.tsx` (nová /welcome cesta + ochrana appky), `src/components/Layout.tsx` (profil
v sidebar), `src/pages/Settings.tsx` (nová sekcia Account), `src/components/icons.tsx` (3 nové ikonky).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.44`.

## STOP

1. Otvor appku - mala by ťa čakať nová úvodná obrazovka namiesto rovno Dashboardu.
2. Preklikaj si registráciu, profil dole v sidebar, Account settings, odhlásenie.
3. Keď budeš mať čas na tie Firebase kroky z chatu, pošli mi výsledný `firebaseConfig` a spravím skutočné
   prihlasovanie.
4. Čokoľvek nezvyčajné alebo čo chceš inak - pošli mi screenshot alebo mi len napíš, čo zmeniť.
