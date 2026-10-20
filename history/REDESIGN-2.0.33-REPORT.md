# TIQR Manager 2.0.33 — stĺpec Sale v Sales teraz zobrazuje celý kód

## Čo je nové

Presne to, čo si nahlásil na screenshote zo Sales: kód predaja (napr. "SAL-000001") sa v prvom stĺpci tabuľky orezával na "SAL-0001…", aj keď ide o pevný, nikdy sa nemeniaci formát bez skutočného dôvodu na skracovanie. Stĺpec je teraz širší, takže sa zobrazuje celý kód.

## Príčina

Stĺpec Sale mal v `<colgroup>` pevnú šírku 90px (Sales.tsx používa `table-layout: fixed`, presne ako popisuje REDESIGN-1.8.2-REPORT.md). Po odčítaní vnútorného odsadenia bunky (`.td-c` má `px-2`, teda 8px na každú stranu = 16px spolu) ostávalo na text len ~74px. Kód predaja má tvar `SAL-` + 6-miestne číslo (`codes.rs::next_code`, napr. "SAL-000001") - to je 10 znakov, ktoré v tučnejšom reze písma (`font-medium`, 14px) potrebujú približne 80-85px. Bolo to teda tesne pod hranicou, a `truncate` trieda (ktorá tam je zámerne, ako poistka) sa preto spúšťala pri každom riadku, nie len výnimočne.

## Oprava

`src/pages/Sales.tsx`: stĺpec Sale rozšírený z 90px na 120px (`w-[90px]` → `w-[120px]` v `<colgroup>`). `truncate` na Linku ostáva - rovnaká poistka, akú má aj `overflow-x-auto` na celej tabuľke - teraz už ale reálne nemá dôvod sa spustiť (spustila by sa len keby počítadlo predajov niekedy prerástlo 6 číslic).

Vedľajší efekt, ktorý netajím: keďže je to `table-layout: fixed` s presne odmeraným rozpočtom šírky (pozri komentár v kóde), pevných 10 stĺpcov teraz spolu zaberá 758px namiesto 728px. Na úplne najužšom podporovanom okne appky (1080px) to znamená, že stĺpcu Event ostáva 50px namiesto pôvodných 80px - v praxi by si to spozoroval len na výrazne zmenšenom okne, keďže Event má vlastný `truncate` + title tooltip a tabuľka má `overflow-x-auto` ako posledná poistka. Ak by ti to niekedy prekážalo, viem to doladiť (napr. ubrať trochu z inej, menej dôležitej kolónky namiesto z Event) - povedz.

## FOUND BUT NOT TOUCHED

Rovnaký základný problém (10-znakový kód, `truncate`, tesná šírka) má s vysokou pravdepodobnosťou aj stĺpec **Order** na stránkach **Orders.tsx** a **Tickets.tsx** - obe majú kód v tvare "ORD-000001" (rovnako 10 znakov) v stĺpci širokom len 92px, s tou istou `truncate` triedou na bunke. Nešiel som do toho, keďže si sa pýtal konkrétne na Sales - daj vedieť, ak to mám opraviť rovnako aj tam (vyzerá to na identický fix, len iné číslo).

## Testy a build

Táto zmena sa netýka žiadneho Rust súboru - `cargo test` je teda bez zmeny oproti 2.0.32 (494 testov, z toho 491 spustených/passed, 3 ignored). Frontend overený staticky (sandbox nemá funkčný `npm ci`/`npm run build`/skutočný Playwright - žiadny prístup k npm/crates registru, rovnaké obmedzenie ako v predchádzajúcich kolách): `Sales.tsx` prešiel syntaktickou kontrolou cez samotný `typescript` balík (`ts.transpileModule`, zachytí nevyvážené zátvorky/JSX) - čisto, žiadne diagnostiky. Aritmetika šírky stĺpcov prepočítaná ručne a krížovo overená proti existujúcemu komentáru v kóde (728px→758px, 80px→50px). JSON súbory (`package.json`, `package-lock.json`, `tauri.conf.json`) overené cez `JSON.parse`, že ostali validné. Skutočný vizuálny render (ako appka naozaj vyzerá na tvojom Windows PC) si prosím skontroluj sám po `1-CLICK-UPDATE.bat` - v tomto sandboxe to overiť nemôžem.

## Zmenené súbory

**Frontend (1 súbor):** `src/pages/Sales.tsx` - šírka stĺpca Sale (90px → 120px) + aktualizovaný komentár nad `<colgroup>` (staré čísla 728px/80px nahradené aktuálnymi 758px/50px, pridaná poznámka k 2.0.33).

**Verzia (8 miest):** `package.json`, `package-lock.json` (2×), `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version`), `1-CLICK-UPDATE.bat` - všetkých na `2.0.33`.

## Čo sa NEMENILO

Žiadny Rust súbor, žiadna migrácia, žiadna iná stránka ani stĺpec (vrátane Order stĺpca na Orders/Tickets - pozri FOUND BUT NOT TOUCHED vyššie). Ostatných 9 stĺpcov v Sales tabuľke, filter riadok, súhrnný riadok, hlavička - nič z toho sa nedotklo.

## STOP

2.0.33 hotové. Skontroluj:

1. Na Sales - kód predaja (napr. "SAL-000001") by sa mal zobrazovať celý, bez "…" na konci.
2. Zvyšok stránky (filtre, súhrn, ostatné stĺpce) by mal vyzerať presne ako v 2.0.32 - nič iné sa nemenilo.
3. Ak chceš, nech opravím rovnaký problém aj na Orders/Tickets (stĺpec Order) - napíš.
