# TIQR Manager 2.0.76 — Upozornenia na desktop, email a Pushover

## Čo som pridal

Toto je druhá (a väčšia) polovica toho, čo si chcel: *"chcem tiez nejake pushup notifikacie, najlepsie aj do mobilu, no len tie najviac prioritne"*. Zvonček na Dashboarde (2.0.75) len **ukazuje** tie isté 4 veci v appke - táto verzia ich vie aj **poslať von**, aj keď appku práve nemáš otvorenú pred sebou.

V Settings pribudla nová sekcia **Notifications** s tromi nezávislými kanálmi - môžeš zapnúť ktorýkoľvek, všetky, alebo žiadny:

- **Desktop** - systémové upozornenie na počítači, kým je appka spustená. Žiadne nastavovanie, len zapnúť.
- **Email** - potrebuje SMTP údaje (napr. Gmail: `smtp.gmail.com`, port 587, tvoj email ako username, a "heslo pre aplikácie" - bežné heslo do Gmailu tu Google nepustí).
- **Pushover** - upozornenie priamo do mobilu. Potrebuješ zadarmo appku Pushover a na pushover.net si vyrobiť "user key" a "application token" - tie sa vložia do appky.

Každý kanál má tlačidlo **"Send test"**, ktoré hneď povie, či to naozaj prešlo - nie je to tiché ako pravidelná kontrola.

Appka odteraz každých 30 minút (a raz hneď po spustení) potichu skontroluje tie isté 4 veci, čo aj zvonček - nezaplatené platby, čakajúce predaje, tikety bez ceny, blížiace sa eventy - a ak niečo treba, pošle to na všetky kanály, ktoré máš zapnuté. Táto kontrola nikdy nič nezobrazí ani nespadne, keby bol napr. email zle nastavený alebo si offline - jednoducho to potichu preskočí a skúsi znova o 30 minút.

## Rozhodnutia, ktoré som urobil za teba (over si, či to sedí)

**"Len tie najviac prioritné" som vyriešil dvoma spôsobmi:**

1. Každá zo 4 kategórií sa pošle **maximálne raz za kalendárny deň** - aj keby appka bežala celý deň a kontrolovala každých 30 minút, o tú istú vec ťa neotravuje opakovane.
2. Blížiace sa eventy majú navyše **vlastný prah 3 dni** - zvonček na Dashboarde ich ukazuje už 14 dní vopred (ako doteraz), ale von (email/Pushover/desktop) sa pošlú, až keď je najbližší event už len 3 dni ďaleko alebo bližšie. Event o 2 týždne je fajn vidieť pri pohľade na Dashboard, ale netreba kvôli nemu budiť telefón.

Ak by si to chcel inak (napr. kratší/dlhší prah, alebo posielať pri každej kontrole bez ohľadu na deň), stačí povedať - je to jedna konštanta v kóde.

**Heslá a kľúče** (SMTP heslo, Pushover kľúče) appka ukladá lokálne u teba v počítači, rovnako ako doteraz ukladá napr. prihlásenie cez Google - nie sú nijak extra šifrované, ale nikdy sa neposielajú nikam von okrem toho, na čo sú určené (SMTP server / Pushover), a appka ich nikdy neukáže naspäť v Settings - políčko je vždy prázdne, s poznámkou že niečo už je uložené, a necháš ho prázdne, ak nechceš meniť.

## Čo musíš vyskúšať ty - ja to odtiaľto neviem overiť naostro

Toto prostredie, v ktorom appku programujem, sa nevie pripojiť na skutočný email server ani na Pushover (rovnaké obmedzenie ako pri Google Sheets/kurzoch mien predtým) - takže toto potrebujem, aby si vyskúšal ty na svojom počítači:

1. V Settings → Notifications zapni Email, vyplň skutočné SMTP údaje a klikni **Send test** - over, že ti email naozaj príde.
2. Zapni Pushover, vlož skutočný user key + token, klikni **Send test** - over, že ti príde push do telefónu.
3. Zapni Desktop, klikni **Send test** - over, že sa ukáže systémové upozornenie.
4. S reálnymi dátami, ktoré appka označí za "attention" (nezaplatená platba, blížiaci event...), počkaj na najbližšiu 30-minútovú kontrolu (alebo reštartni appku, tá kontrola beží aj hneď po spustení) a over, že upozornenie naozaj príde na zapnuté kanály.
5. Skús to znova čoskoro potom - to isté upozornenie by sa **nemalo** poslať druhý raz v ten istý deň.
6. Over, že event vzdialenejší než 3 dni sa síce ukazuje na zvončeku, ale (zatiaľ) nepošle upozornenie von.

## Obmedzenie, o ktorom treba vedieť

Upozornenia fungujú, len kým appka beží - nie je tam (zatiaľ) žiadna služba na pozadí ani ikonka pri hodinách, ktorá by appku držala nažive aj po zavretí okna. Ak by si chcel, aby ťa appka vedela upozorniť, aj keď je úplne zavretá, je to samostatná a väčšia téma (ikonka v systéme, spúšťanie s Windowsom) - povedz, ak to bude aktuálne.

## Čo som overil

```
cargo test --lib   -> 729 testov, 0 zlyhaní, 3 ignorované (26 nových)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Skutočné odoslanie emailu/Pushover správy som overiť nevedel (pozri vyššie) - kód okolo toho (kedy sa má niečo poslať, ako znie správa, ukladanie nastavení, že sa heslo neopakuje druhýkrát) je ale plne otestovaný.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/013_notifications.sql` — nová tabuľka na "toto sa už dnes poslalo".
- `src-tauri/src/commands/notifications.rs` — nový súbor, celá logika (čo poslať, komu, kedy).
- `src-tauri/src/commands/dashboard.rs` — výpočet upozornení vytiahnutý nabok, aby ho vedela použiť aj táto nová kontrola (zvonček aj email/Pushover teraz čerpajú z jedného miesta).
- `src-tauri/src/models.rs`, `src-tauri/src/db.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml` — prepojenie nového kódu do appky + dve nové knižnice (upozornenia na desktop, odosielanie emailov).

**Frontend:**
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania pre 6 nových príkazov.
- `src/components/Layout.tsx` — pravidelná kontrola každých 30 minút.
- `src/pages/Settings.tsx` — nová sekcia Notifications.

**Verzia (9 miest v 7 súboroch):** `2.0.76`.

## To bol posledný kus z tvojej správy

Týmto je hotové všetkých 6 vecí z tej väčšej správy: oddelené dáta pre každý účet (2.0.72), zjednodušené Settings → Lookups (2.0.73), animácie (2.0.74), zvonček na Dashboarde (2.0.75) a teraz upozornenia von (2.0.76). Ostáva už len 2FA a overovanie emailu pri registrácii, na to sa vrátim, až to budeš chcieť riešiť.
