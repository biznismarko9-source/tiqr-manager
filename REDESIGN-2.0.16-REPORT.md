# TIQR Manager 2.0.16 — Pulls a Orders & Sales vedľa seba, Currency preč

## 1. Layout

**Sign in with Google** je teraz jediná karta na celú šírku hore, presne ako predtým. Pod ňou sú **Pulls** a **Orders & Sales** vedľa seba (na užšej obrazovke sa samy poskladajú pod seba, aby sa nič nezmestilo zle).

## 2. Currency je preč

Pole Currency som z oboch kariet úplne odstránil. Predtým, než som to spravil, som si to overil v kóde - a tu je dôležitý rozdiel medzi Pulls a Orders & Sales, čo stojí za vysvetlenie:

- **Orders & Sales** - mal si pravdu úplne, appka menu naozaj číta priamo z riadku v hárku (stĺpec currency) automaticky, picker bol už len záloha pre riadok, čo by mal ten stĺpec prázdny.
- **Pulls** - tvoj Pulls hárok stĺpec s menou vôbec nemá, nikdy nemal. Ten picker bol tam jediný spôsob, ako appka vedela, v akej mene tvoje pully sú.

Preto som sa ťa spýtal, ako to vyriešiť - vybral si **"Všade EUR"**, takže presne to appka teraz robí: EUR je pevne nastavené, appka ho posiela na pozadí bez toho, aby si čokoľvek vyberal. Pre Orders & Sales sa nič nemení (riadok stále vyhráva, ak má svoju menu vyplnenú) - mení sa len to, čím sa doplní prázdna bunka (predtým výber, teraz vždy EUR). Pre Pulls je teraz každý riadok EUR.

Backendová (peniazová) logika, čo menu spracúva a validuje, som sa vôbec nedotkol - len som z Settings odstránil samotný výber, appka teraz vždy posiela EUR namiesto toho, aby si to volil ty. Ak by si niekedy potreboval iné pully alebo hárky v inej mene než EUR, daj vedieť - dá sa to jednoducho vrátiť alebo prerobiť na inú pevnú menu.

## 3. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 348 passed, 0 failed (bez zmeny - toto bola len frontendová úprava)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.16 build" v hlavičke)
```

## 4. Zmenené súbory

**Zmenené:** `src/pages/Settings.tsx` (layout Pulls/Orders & Sales vedľa seba, Currency pole úplne odstránené z oboch kariet, appka teraz vždy posiela EUR na pozadí)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.16`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.16 hotové a overené (348/348 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj Settings -> Integrations:

1. Pulls a Orders & Sales vedľa seba, Sign in with Google hore cez celú šírku.
2. Na žiadnej z kariet už nie je vidieť pole Currency.
3. Save/Connect na oboch kartách funguje ako doteraz (na pozadí sa teraz vždy pošle EUR).

Napíš mi, či to takto sedí.
