# TIQR Manager 2.0.45 — Login/Register, krok 2a z 2b: skutočné prihlásenie cez email+heslo

## Čo si mi napísal

Poslal si mi svoj skutočný Firebase config z konzoly (projekt `tiqr-manager-b890a`) s otázkou *"myslis
nieco z tohto?"* - presne to. Znamená to, že si dokončil kroky na Firebase konzole, ktoré som ti dal
v predchádzajúcej správe.

## Čo je nové

Prihlásenie cez **email a heslo teraz naozaj funguje** - napojené na tvoj skutočný Firebase projekt, nie
už len "nanečisto" ako v 2.0.44:

- **Registrácia** vytvorí skutočný účet vo Firebase (uvidíš ho vo Firebase konzole pod Authentication ->
  Users, presne s časom, kedy sa niekto zaregistroval - presne to, čo si chcel vidieť ako dev).
- **Prihlásenie a odhlásenie** fungujú naozaj - heslo sa naozaj overuje, nie je to už "príjme čokoľvek".
- **Zmena mena v Nastaveniach -> Account** sa teraz naozaj uloží a hneď sa prejaví aj v profile dole
  v sidebar.
- Ak niečo zlyhá (zlé heslo, email už existuje, slabé heslo, žiadne internetové pripojenie...), appka
  ukáže zrozumiteľnú anglickú vetu (appka je celá po anglicky, ako doteraz), nikdy nejakú kryptickú chybu
  rovno z Firebase.

**"Continue with Google" tlačidlo je zatiaľ stále len "coming soon"** - vidno ho, ale je vypnuté. To je
krok 2b, ešte len príde (potrebujem od teba jednu vec z Firebase konzoly, keď na to dôjde rad).

## Dôležité - ako som to overoval (prečítaj si, prosím)

Moje vývojové prostredie (kde appku staviam a testujem) nemá priamy prístup na internet k skutočným
Firebase serverom - je to obmedzenie prostredia, v ktorom pracujem, nie chyba v appke. Skúšal som sa
naozaj zaregistrovať cez tvoj skutočný projekt a spojenie jednoducho neprešlo (appka to správne rozpoznala
a ukázala hlášku o probléme s pripojením - čo je vlastne pekný dôkaz, že aj toto funguje správne).

Namiesto toho som appku dôkladne otestoval cez **oficiálny Firebase nástroj na lokálne testovanie**
("emulátor") - je to ich vlastný softvér, ktorý sa správa presne ako skutočný Firebase, len bežia lokálne
u mňa. Prešiel som celý tok automatizovane (Playwright, so screenshotmi, ktoré som si aj reálne pozrel):
registrácia, odhlásenie, zlé heslo (správne odmietnuté so zrozumiteľnou hláškou), správne prihlásenie,
zatvorenie a znova otvorenie appky počas prihlásenia (aby neproblikla prihlasovacia obrazovka), zmena mena,
aj pokus zaregistrovať ten istý email druhýkrát (správne odmietnuté). Všetko prešlo bez jedinej chyby.

Takže: kód je poriadne otestovaný a mal by fungovať na 100 %, ale úplne posledný krok - naozaj sa
zaregistrovať v appke u teba na počítači, cez tvoj skutočný Firebase projekt - som nemohol spraviť ja
osobne. Prosím ťa o toto:

1. Nainštaluj si 2.0.45 a skús sa naozaj zaregistrovať (svoj email, hocijaké heslo).
2. Ak to prejde a uvidíš svoje meno dole v sidebar - super, funguje to, over si aj vo Firebase konzole pod
   Authentication -> Users, že sa tam objavil tvoj účet.
3. Ak by náhodou vyskočila chyba typu "This sign-in method isn't turned on yet" - znamená to, že vo
   Firebase konzole pod Authentication -> Sign-in method treba zapnúť "Email/Password" (mal by byť
   zapnutý už z krokov, čo si robil predtým, ale keby náhodou nie).
4. Čokoľvek zvláštne uvidíš - pošli mi screenshot, opravím to hneď.

## Čo teraz urobiť

1. Nainštaluj 2.0.45.
2. Zaregistruj sa naozaj (pozri vyššie).
3. Skús sa odhlásiť a znova prihlásiť s tým istým heslom.
4. Skús v Nastaveniach -> Account zmeniť meno a ulož.
5. Daj mi vedieť, či všetko prešlo hladko - potom môžeme ísť na krok 2b (skutočné "Continue with Google").

## Testy a build

```
npx tsc -b        -> 0 chýb
npm run build     -> OK
cargo test --lib  -> 560 testov, všetky prešli (žiadny Rust súbor sa túto verziu nemenil)
```

Táto zmena je len frontend (React/TS) - žiadny Rust súbor sa nemenil. Naviac plný automatizovaný tok cez
Firebase emulátor (popísané vyššie) - bez jedinej neočakávanej chyby v konzole.

## Zmenené súbory

**Nové (2):** `src/lib/firebase.ts` (pripojenie k tvojmu Firebase projektu), `src/lib/firebaseErrors.ts`
(preklad Firebase chýb do zrozumiteľných viet).

**Upravené (5):** `src/lib/auth.tsx` (celé prepísané na skutočné Firebase volania namiesto "nanečisto"),
`src/App.tsx`, `src/pages/Welcome.tsx`, `src/components/Layout.tsx`, `src/pages/Settings.tsx` (drobné
úpravy - správne čakanie kým sa obnoví prihlásenie po reštarte appky, spracovanie chýb).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.45`.

## STOP

1. Nainštaluj 2.0.45 a skús sa naozaj zaregistrovať svojím emailom - toto je ten jeden krok, ktorý som
   nemohol overiť ja sám (vysvetlené vyššie).
2. Skontroluj vo Firebase konzole (Authentication -> Users), že sa tam tvoj účet objavil.
3. Skús sa odhlásiť, znova prihlásiť, a v Nastaveniach -> Account zmeniť meno.
4. Napíš mi, či všetko sedí - potom pôjdeme na skutočné "Continue with Google" (krok 2b).
