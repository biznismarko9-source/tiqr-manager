# TIQR Manager 2.0.29 — Pulls: Date, Platform aj Warning sa už neorezávajú

## Čo je nové

Presne to, čo si nahlásil na screenshote z **Given** záložky v Pulls:

- **Date** — predtým orezané ("23. 10. 2..."), teraz vždy celý dátum ("Oct 23, 2026").
- **Platform** — predtým orezané ("Ticketma..."), teraz celý názov ("Ticketmaster").
- **Warning** — predtým sa "43d overdue" tesnalo a orezávalo, teraz je celé aj s ikonkou.

Na záložke **Received** som rovno opravil aj tam orezaný **Date** stĺpec (mal presne tú istú chybu, len si ho na screenshote neposielal - je to ale ten istý stĺpec, tá istá príčina).

Ako presne som to spravil: stĺpce som zväčšil (Date/Platform 84px → 120px, Warning 76px → 130px), nezmenšoval som písmo ani nič nezhustil - presne to isté rozhodnutie, aké si už raz spravil pre stĺpce Seats a More info v tejto istej tabuľke (tie sú kvôli tomu odvtedy dosť široké). Keď sa tabuľka na užšom okne celá nezmestí, jednoducho sa objaví vodorovný posuvník len pri tejto tabuľke - presne tak to už predtým fungovalo aj pre Seats/More info, len som to isté pravidlo teraz predĺžil aj na tieto tri stĺpce.

## Ako presne to funguje pod kapotou

Pri zväčšovaní stĺpcov som narazil na skutočnú chybu, ktorá tu asi bola už dávnejšie, len sa nikdy neprejavila: keď sú v tabuľke pevné šírky stĺpcov väčšie než dostupné miesto, stĺpec "Event" (jediný bez pevnej šírky - naťahuje sa podľa zvyšného miesta) sa namiesto vodorovného posúvania celý stlačil skoro na nič - meno eventu sa prestalo dať prečítať, namiesto toho aby sa tabuľka len rozšírila a objavil sa posuvník. Opravil som to pridaním minimálnej šírky priamo na `<table>` (1320px pre Given, 1080px pre Received - súčet všetkých pevných stĺpcov plus rozumné minimum pre Event), takže teraz sa tabuľka vždy buď zmestí celá, alebo sa naozaj rozšíri a objaví sa posuvník - nikdy sa už nestlačí meno eventu na nečitateľné.

Čisto frontendová zmena, jeden súbor (`src/pages/Pulls.tsx`) - žiadna zmena v databáze, žiadna zmena v Rust kóde, žiadne nové API volanie.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - táto oprava sa Rust kódu vôbec netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.29 build" v hlavičke)
```

Vizuálne som si to overil cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) - obe záložky (Given aj Received), svetlý aj tmavý režim, na širokom okne (všetko vidno naraz) aj na užšom okne (vodorovný posuvník funguje, nič sa nestláča), s reálnymi hodnotami z tvojho screenshotu (kevin/betlanovce, tomas/topolky s "43d overdue", sojky/ruhanovce s "Today!") plus jeden zámerne extrémny testovací riadok (veľmi dlhý názov platformy, "402d overdue"), a že tlačidlo "Delete"/výber pri hromadnom mazaní (z 2.0.28) so širšími stĺpcami stále vyzerá a funguje správne.

## Zmenené súbory

**Frontend:**
- `src/pages/Pulls.tsx` - šírky stĺpcov Date/Platform/Warning (Given) a Date (Received), plus `min-width` na oboch tabuľkách (oprava chyby so stláčaním stĺpca Event)

**Verzia (7 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` - všetkých 7 na `2.0.29`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.29 hotové a overené (491/491 testov, čisté `tsc`/`build`, vizuálne overené). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Pulls → Given - skontroluj, že Date, Platform aj Warning sú teraz vždy celé vidno (skús si to aj na svojich reálnych, dlhších dátach - najmä pri platformách s dlhším názvom).
2. Pulls → Received - Date stĺpec tiež celý.
3. Ak je okno appky užšie, over si, že sa pri tejto tabuľke objaví vodorovný posuvník namiesto toho, aby sa niečo orezávalo alebo strácalo.
4. Ak by si radšej videl kratší formát dátumu (napr. "15 Aug 26" namiesto "Oct 23, 2026") namiesto širšieho stĺpca, appka už takýto kratší formát inde má (Sales) - napíš mi, viem to takto zjednotiť aj tu.
