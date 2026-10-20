# TIQR Manager 2.0.31 — obsah teraz vyplní celé okno, aj na maximalizovanom

## Čo je nové

Presne to, čo si ukázal na tých dvoch porovnaniach - keď máš appku maximalizovanú (na veľkom monitore), obsah stránky sa už nezastaví na neviditeľnej hranici a nenechá prázdny pás miesta vpravo. Vyplní sa to úplne rovnako, ako to appka už robila na menšom, nemaximalizovanom okne.

Platí to na každej stránke, nielen na tej, čo si poslal na screenshote (Dashboard) - je to jedna spoločná zmena v layoute, ktorý majú všetky stránky pod sebou.

## Ako presne to funguje pod kapotou

Príčina bola jednoduchá: obsah stránky mal nastavenú maximálnu šírku 1400px a od tej šírky vyššie sa vycentroval so zvyšným miestom po oboch stranách namiesto toho, aby sa roztiahol. Na bežnom, menšom okne to nebolo vidno, lebo okno samo osebe nikdy nebolo také široké, aby appka na tento strop vôbec narazila - preto to na menšom okne vyzeralo v poriadku. Na veľkom/maximalizovanom okne to ale narazilo a bolo to vidno presne tak, ako si popísal.

Jednoducho som ten strop odstránil - obsah sa teraz vždy roztiahne na celú dostupnú šírku, nech je okno akokoľvek veľké. Overil som si to aj číselne (nie len okom) - zmeral som v prehliadači, že šírka obsahu presne zodpovedá dostupnému miestu, s nulovou medzerou, na simulovanom 1920px okne. Menšie okno (1440px) vyzerá presne rovnako ako predtým - tam ten strop aj predtým nikdy neprekážal, takže sa tam nič nemenilo.

Tabuľky (napríklad Pulls, čo sme upravovali predtým) a mriežka štatistík na Dashboarde len dostanú o niečo viac priestoru na veľkom monitore - nič sa tým nepokazí, len sa to krajšie roztiahne.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - tato oprava sa Rust kódu vôbec netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.31 build" v hlavičke)
```

Overené aj cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) - na 1920px okne (presne ten prípad, čo si nahlásil), svetlý aj tmavý režim, a porovnané s 1440px oknom, aby som si overil, že sa tam nič nezmenilo k horšiemu.

## Zmenené súbory

**Frontend:**
- `src/components/Layout.tsx` - odstránená maximálna šírka obsahu (`max-w-[1400px]`)

**Verzia (7 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` - všetkých 7 na `2.0.31`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.31 hotové a overené (491/491 testov, čisté `tsc`/`build`, overené aj číselne aj vizuálne). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Maximalizuj appku na celú obrazovku (alebo na akokoľvek veľké okno máš) a skontroluj, že obsah teraz vyplní celú šírku bez medzery vpravo, na Dashboarde aj na ostatných stránkach.
2. Skontroluj, či sa ti to takto páči aj na fakt veľmi širokom okne - ak by ti niektorá stránka prišla "príliš roztiahnutá", napíš mi, viem tam vrátiť nejaký rozumný strop.
