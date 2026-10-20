# TIQR Manager 2.0.77 — Zjednodušenie notifikácií

## Čo som spravil podľa tvojej poslednej správy

Presne ako si napísal: *"chcem, aby do toho pushover sa daval len user key nic ine a do email notifications len mail a budu posielane automaticke maily"* a potom *"alebo zatial dajme len pushover, dashboard a desktop notifications, email zatial odstranme"*.

- **Email som úplne odstránil.** Žiadne SMTP nastavenia, žiadne políčka na server/heslo/adresu - v Settings → Notifications už email vôbec nie je. Z appky zmizla aj knižnica na posielanie emailov (lettre), ktorú 2.0.76 pridala.
- **Pushover teraz žiada len jednu vec - tvoj user key.** Predtým bolo treba vyplniť aj "API token" appky - to som presunul tak, aby appka mala **svoj vlastný, zabudovaný token**, rovnako ako to appka už dnes robí pri Google Sheets synchronizácii (appka má svoj vlastný prístup, ty len povolíš prístup k svojmu účtu). Nižšie v sekcii "Čo ešte treba" presne píšem, čo pre to potrebujem od teba.
- **Zvonček na Dashboarde a desktop upozornenia sú bez zmeny** - fungujú presne ako doteraz.

## Ako to funguje teraz

V Settings → Notifications sú dva prepínače:

1. **Desktop notifications** - systémové upozornenie, kým appka beží. Žiadne nastavovanie.
2. **Pushover (mobile push)** - zapneš, vložíš svoj **user key** (nájdeš ho na pushover.net po prihlásení, na hlavnej stránke) a je to.

Appka to skontroluje každých 30 minút (a raz hneď po spustení) a pošle upozornenie na zapnuté kanály, ak treba - rovnaké 4 veci ako doteraz (nezaplatené platby, čakajúce predaje, chýbajúca cena, blížiaci event do 3 dní). Max. raz za deň na jednu vec, aby to neotravovalo.

## Čo ešte treba - jedna vec od teba, aby Pushover naozaj fungoval

Aby appka vedela posielať cez Pushover, potrebuje svoj vlastný "aplikačný token" (Pushover to vyžaduje od každej appky, čo cezeň niečo posiela - je to iné číslo než tvoj osobný user key). Je to jednoduché a zadarmo:

1. Choď na **pushover.net/apps/build**, prihlás sa (alebo si zaregistruj účet, ak ešte nemáš).
2. Vytvor novú "Application" - stačí jej dať meno, napr. "TIQR Manager", ikonka je voliteľná.
3. Po vytvorení uvidíš **"API Token/Key"** - to skopíruj a pošli mi ho (alebo ho sám pridaj do GitHub repozitára: Settings → Secrets and variables → Actions → New repository secret, meno `PUSHOVER_API_TOKEN`, hodnota ten token).

Kým tento krok nespravíš, appka sa bude tváriť normálne (Desktop notifikácie fungujú, Pushover sa dá zapnúť a uložiť), len tlačidlo "Send test" pri Pushover ti povie "Pushover isn't available in this build" - presne to isté, čo appka dnes hlási pri Google Sheets, kým nebolo nastavené. Akonáhle token pridám/pridáš, ďalší build appky ho už bude mať zabudovaný natrvalo - nie je to niečo, čo by si musel robiť znova.

## Čo som overil

```
cargo test --lib   -> 732 testov, 0 zlyhaní, 3 ignorované (29 v notifications module)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Medzi tými 29 testami je aj jeden špecificky na to, že appka, ktorá mala uložené nastavenia ešte v starom tvare z 2.0.76 (s emailom), sa po tejto zmene nerozbije - jednoducho email časť ignoruje a Pushover/Desktop nastavenia si prečíta normálne.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/src/commands/notifications.rs` — email preč, Pushover token teraz zabudovaný.
- `src-tauri/src/models.rs` — zjednodušené dátové typy (bez emailu, bez Pushover api_token poľa).
- `src-tauri/src/lib.rs` — odstránený príkaz na testovanie emailu.
- `src-tauri/build.rs`, `.github/workflows/build-windows.yml` — nový (voliteľný) `PUSHOVER_API_TOKEN` build secret, rovnaký princíp ako existujúce Google/Anthropic kľúče.
- `src-tauri/Cargo.toml` — odstránená knižnica `lettre` (email).

**Frontend:**
- `src/lib/types.ts`, `src/lib/api.ts` — zjednodušené typy, odstránený `testEmailNotification`.
- `src/pages/Settings.tsx` — sekcia Notifications teraz ukazuje len Desktop a Pushover (user key), bez emailu a bez API tokenu.

**Verzia (9 miest v 7 súboroch):** `2.0.77`.
