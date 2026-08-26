# TIQR Manager 2.0.46 — Login/Register, krok 2b z 2b: skutočné "Continue with Google" (hotovo)

## Čo si mi napísal

*"ide to, podme na google login"* — potvrdil si, že skutočná registrácia cez email+heslo z 2.0.45 funguje, a
chcel si ísť ďalej. Poslal si mi Client ID a Client Secret z nového OAuth klienta, čo si vytvoril v Google
Cloud Console pre projekt `tiqr-manager-b890a` (aj s malou zákrutou — pôvodne si ten projekt nevedel nájsť vo
výbere projektov, vyriešili sme to priamym odkazom).

## Čo je nové

**"Continue with Google" na úvodnej obrazovke teraz naozaj funguje.** Klikneš naň, appka ti otvorí tvoj bežný
prehliadač na skutočnú Google prihlasovaciu stránku (presne rovnaký princíp ako existujúce "Sign in with
Google" v Nastaveniach pre Sheets — len úplne oddelené, tamto sa vôbec netýka Sheets, appka si len pýta tvoje
meno a email), ty sa prihlásiš/potvrdíš, prehliadač ťa presmeruje späť a appka ťa prihlási aj do appky
samotnej. Ak appka čaká na teba v prehliadači, tlačidlo ukáže "Waiting for you to finish in your browser..."
s možnosťou Cancel — presne ako pri Sheets prihlásení.

Ak niekto skúsi "Continue with Google" s emailom, pod ktorým už existuje účet cez email+heslo, appka to
rozpozná a ukáže zrozumiteľnú hlášku namiesto toho, aby vytvorila druhý, zmätočný účet.

Tým je login/register hotové presne tak, ako si to na začiatku chcel: úvodná obrazovka, voľba login/register,
Google aj email+heslo, profil dole v sidebar, Nastavenia → Account, odhlásenie — všetko naozaj funguje, nič
z toho už nie je "nanečisto".

## Dôležité — over si toto ako prvé, keď to budeš skúšať

Presne ako pri pôvodnom "Sign in with Google" pre Sheets (2.0.5): Google projekty defaultne bežia v režime
**"Testing"**, kde sa vedia prihlásiť len emaily, čo si ručne pridal do zoznamu "Test users" pre daný projekt.
Ak pri prvom skutočnom kliknutí na "Continue with Google" uvidíš niečo ako "app not verified" alebo "access
blocked" — nie je to chyba v appke, len treba v Google Cloud Console (projekt `tiqr-manager-b890a`) →
**APIs & Services → OAuth consent screen** pridať seba (a kohokoľvek ďalšieho, kto to skúša) do Test users,
presne ako sme to riešili pri Sheets. Táto časť by mala byť jednoduchšia než pri Sheets — appka si tu pýta
len základné meno+email, nie prístup k Sheets, takže by nemalo byť treba to zdĺhavé Googlovské schvaľovanie s
videom. Uvidíme, keď to skúsiš naostro.

## Ako som to overoval

Rovnaké obmedzenie ako pri Sheets prihlásení (2.0.5) aj pri Firebase emailovom prihlásení (2.0.45): moje
vývojové prostredie sa nevie pripojiť na skutočné Google prihlasovacie servery, takže samotné kliknutie cez
skutočný prehliadač som nemohol vyskúšať ja sám. Overil som teda všetko, čo sa overiť dalo:

- Celý kód prešiel čisto cez `tsc`/`build` aj cez appkine testy (567 testov, o 7 viac než v 2.0.45 — nové
  testy pre novú časť kódu).
- Automatizovane (Playwright) som prešiel úvodnú obrazovku a potvrdil, že tlačidlo "Continue with Google"
  vyzerá a správa sa správne, existujúci email+heslo tok naďalej funguje bez zmeny, a appka nehlási žiadne
  neočakávané chyby.
- Samotnú výmenu kódu za prihlásenie na `accounts.google.com` som starostlivo skontroloval podľa toho, ako už
  funguje overené "Sign in with Google" pre Sheets — je to doslova ten istý, už raz overený mechanizmus,
  len s inými prihlasovacími údajmi a užším rozsahom (len meno+email, nič iné).

Skús to teda ty ako prvý naozaj naostro, a daj mi vedieť, ako to dopadlo.

## Čo teraz urobiť

1. Nainštaluj 2.0.46.
2. Na úvodnej obrazovke skús kliknúť "Continue with Google".
3. Ak vyskočí "app not verified"/"access blocked" — pozri sekciu vyššie (Test users v Cloud Console).
4. Ak to prejde — super, si prihlásený aj cez Google. Skús sa odhlásiť a prihlásiť znova.
5. Daj mi vedieť, ako to dopadlo.

## Testy a build

```
cargo test --lib  -> 567 testov, všetky prešli (z 2.0.45: 560 + 7 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

## Zmenené súbory

**Nové (backend):** `src-tauri/src/commands/firebase_google_auth.rs` (3 nové príkazy pre "Continue with
Google" — bez akéhokoľvek ukladania do appkinej databázy, o samotné prihlásenie sa odteraz stará Firebase).

**Upravené (backend):** `src-tauri/src/google_oauth.rs` (rozšírené, aby ho vedeli použiť obe prihlasovacie
cesty — Sheets aj appka), `src-tauri/src/commands/google_auth.rs` (drobná úprava, aby druhá cesta mohla znova
použiť už hotovú "Cancel" logiku), `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`,
`.github/workflows/build-windows.yml` (dva nové GitHub secrets — pozri nižšie).

**Upravené (frontend):** `src/lib/auth.tsx` (`loginWithGoogle` teraz naozaj funguje), `src/lib/api.ts`,
`src/lib/types.ts`, `src/lib/firebaseErrors.ts` (nová zrozumiteľná hláška pre prípad duplicitného Google
účtu), `src/pages/Welcome.tsx` (tlačidlo Google teraz naozaj klikateľné, s Cancel počas čakania).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.46`.

## Dva nové GitHub secrets — už si ich pridal, len pre poriadok

| Meno | Hodnota |
|---|---|
| `FIREBASE_GOOGLE_OAUTH_CLIENT_ID` | Client ID, čo si mi poslal |
| `FIREBASE_GOOGLE_OAUTH_CLIENT_SECRET` | Client Secret, čo si mi poslal |

Bez týchto dvoch by appka fungovala úplne rovnako, len by tlačidlo "Continue with Google" ostalo vypnuté —
presne tá istá "nič sa nepokazí, len funkcia chýba" logika ako pri všetkých ostatných secrets v tomto
projekte.

## STOP

1. Nainštaluj 2.0.46 (spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build).
2. Skús "Continue with Google" naozaj naostro — toto je ten jeden krok, čo som nemohol overiť ja sám.
3. Ak vyskočí "app not verified" — priprav sa na krátky krok v Cloud Console (Test users), presne ako pri
   Sheets prihlásení.
4. Daj mi vedieť, ako to dopadlo — tým je celé login/register hotové, presne podľa toho, čo si na začiatku
   chcel.
