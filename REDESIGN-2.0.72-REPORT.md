# TIQR Manager 2.0.72 — Každý účet má teraz svoje vlastné, oddelené dáta

## Čo si nahlásil

*"ale kazdy ucet ma mat ine data, teraz som sa prihlasil ceu druhy ucet a mal som rovnake data, to treba
zmenit"*

Pýtal som sa, či je to očakávané (rôzni ľudia bežne používajú vlastné počítače, takže sa to prejavilo len
preto, že si oba účty skúšal na jednom stroji), alebo či naozaj chceš oddelené dáta aj keď viac účtov zdieľa
jeden počítač — vybral si **"Nie, chcem oddelené dáta na jednom počítači"**, výslovne s vedomím, že je to
väčšia zmena než čokoľvek doteraz v tejto session.

## Ako to funguje teraz

Doteraz mala celá appka presne jeden spoločný súbor s dátami (`tiqr-manager.sqlite3`) — Firebase prihlásenie
kontrolovalo len KTO sa dostane dnu, nikdy KTORÉ dáta uvidí. Teraz:

- **Tvoj vlastný účet (a hocijaký iný účet, čo už existoval predtým, než táto zmena vyšla) používa presne
  ten istý súbor ako doteraz** — nič sa nekopíruje, nič sa nemigruje, riziko že by "niekto iný" omylom
  prevzal tvoje dáta je nulové.
- Každý účet, čo schváliš odteraz nanovo, dostane pri prvom použití úplne nový, prázdny súbor — vlastný, len
  pre neho.
- Prepínanie medzi súbormi robí appka sama, automaticky, hneď po prihlásení a schválení — nič nemusíš robiť
  ručne.

Technicky: namiesto pridávania "čí je tento riadok" stĺpca do každej z ~100 existujúcich databázových
otázok v appke (rizikové — stačilo by jedno zabudnuté miesto a dáta by sa mohli premiešať medzi účtami),
appka jednoducho **prepne, ktorý celý súbor má práve otvorený**. Každá existujúca funkcia v appke funguje
úplne nezmenená — nevie a ani nepotrebuje vedieť, čí dáta práve vidí.

## Dôležitý vedľajší efekt — platformy, kategórie, Sheets pripojenie sú teraz tiež "per účet"

Toto zámerne nezakrývam: nastavenia (platformy, dodávatelia, kategórie udalostí, farebný motív, pripojenie
na Google Sheets) boli doteraz uložené v tom istom jednom súbore, takže teraz sú **aj ony** viazané na
konkrétny účet. Nový schválený účet preto dostane naozaj čisto prázdny priestor — bude si musieť sám
nastaviť vlastné platformy/kategórie/Sheets pripojenie, presne tak, ako keby appku inštaloval prvýkrát.
Tvojho vlastného účtu sa to netýka — všetko, čo máš nastavené, ostáva presne tak, ako je.

## Čo robiť, ak appka niekedy ukáže "Couldn't open your data"

Nová obrazovka (`DatabaseError`) sa zobrazí len vo výnimočnom prípade, že sa appke nepodarí otvoriť/vytvoriť
súbor pre daný účet (napr. plný disk, chýbajúce oprávnenia na priečinok). Ukáže presné chybové hlásenie a
tlačidlo Log out. Ak ju niekedy uvidíš, napíš mi presne to, čo tam píše — bez toho sa to nedá diagnostikovať.

## Menšie doplnky v Settings

- Pri "Database file:" v Settings → Data teraz vidíš aj email prihláseného účtu, ku ktorému ten súbor patrí.
- Potvrdzovacie okno pri "Restore from backup" teraz menom uvádza, ktorého účtu dáta prepíšeš — keďže teraz
  môže na jednom počítači existovať viac účtov, chcel som znížiť riziko, že niekto omylom obnoví zálohu
  patriacu inému účtu (appka totiž len overuje, že súbor je platná záloha TIQR appky, nie čí presne je).

## Čo som overil, a čo si musíš vyskúšať sám

```
cargo test --lib   -> 703 testov, 0 zlyhaní, 3 ignorované (+10 nových presne na toto: že "účet B" nikdy
                       nevidí dáta "účtu A" a naopak, že prepnutie na ten istý súbor nič nerobí navyše
                       (no-op), že nový súbor dostane všetkých 12 migrácií rovnako ako ten pôvodný)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Samotné prepínanie medzi reálnymi Firebase účtami sa nedá odskúšať odtiaľto (nemám prístup k tvojmu Firebase
projektu naživo) — preto, prosím, priamo po aktualizácii over sám, presne v tomto poradí:

1. Prihlás sa svojím reálnym účtom → over, že vidíš presne to isté, čo doteraz (skús pár známych
   objednávok/lístkov), a že cesta v Settings → Data je rovnaká ako pred aktualizáciou.
2. Zaregistruj jeden skúšobný účet (presne ako pri 2.0.71) a schváľ ho vo Firebase konzole.
3. Prihlás sa tým skúšobným účtom → over, že appka je úplne prázdna (žiadne udalosti/objednávky) a že cesta
   v Settings → Data je INÁ, než mal tvoj reálny účet.
4. Pridaj si tam jednu skúšobnú udalosť (čokoľvek, len na test).
5. Odhlás sa, prihlás sa naspäť svojím reálnym účtom → over, že tá skúšobná udalosť sa NIKDE neukazuje a
   všetko ostatné tvoje je netknuté.
6. Prihlás sa ešte raz skúšobným účtom → over, že tá skúšobná udalosť tam stále je (dokazuje to, že to
   funguje aj pri opakovanom prepínaní, nie len prvýkrát).

Ešte predtým, než čokoľvek z vyššie uvedeného spravíš: odporúčam ručne, mimo appky, skopírovať si niekam
bokom aktuálny súbor `tiqr-manager.sqlite3` (appka ti presnú cestu ukáže v Settings → Data) — čisto ako
poistku navyše, nezávisle od zálohovania v appke samotnej, vzhľadom na to, o aké dáta ide.

## Zmenené súbory

**Backend:**
- `src-tauri/src/db.rs` — nové `AppState.db_path`, `sanitize_uid_for_filename`, `resolve_user_db_path`.
- `src-tauri/src/commands/database.rs` (nové) — `switch_active_database`, prepína živé pripojenie na
  správny súbor.
- `src-tauri/src/commands/backup.rs` — `restore_database` teraz obnovuje ten súbor, čo je práve aktívny
  (predtým vždy ten pôvodný spoločný).
- `src-tauri/src/commands/app_info.rs` — "Database file" v Settings teraz ukazuje skutočne aktívny súbor
  (predtým vždy ukazoval ten pôvodný spoločný, aj keby bol aktívny iný).
- `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs` — zapojenie nového modulu.

**Frontend:**
- `src/lib/auth.tsx` — po prihlásení a schválení appka automaticky prepne na správny súbor (nové
  `dbReady`/`dbError`).
- `src/App.tsx` — nová obrazovka `DatabaseError` sa zobrazí namiesto appky, ak sa prepnutie nepodarí.
- `src/pages/DatabaseError.tsx` (nové).
- `src/lib/api.ts`, `src/lib/types.ts` — nové `switchActiveDatabase`/`DatabaseSwitchOutcome`.
- `src/pages/Settings.tsx` — email pri "Database file", presnejšie znenie pri "Restore from backup".

**Verzia (8 miest):** `2.0.72`.

## STOP — over si to podľa krokov vyššie, prosím

Toto je zatiaľ najväčšia a najcitlivejšia zmena tejto session — ide o tvoje reálne obchodné dáta. Než tomu
naplno začneš dôverovať, prosím prejdi si postupnosť 6 krokov vyššie presne v tom poradí (najmä kroky 1 a 5
— že tvoje vlastné dáta ostali úplne netknuté). Ak čokoľvek nesedí, napíš mi presne čo, skôr než budeš appku
ďalej používať naostro.
