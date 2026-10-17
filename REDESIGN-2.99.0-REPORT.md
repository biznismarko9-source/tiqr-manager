# TIQR Manager 2.99.0 — Discord Test Webhook

Pýtal si funkčný test webhooku do Discordu. Je hotový a **naozaj posiela
požiadavku cez backend a zobrazuje skutočný výsledok** — nie atrapu.

Najskôr tri veci, ktoré musím povedať nahlas, lebo tvoj prompt predpokladal
inú architektúru, než aká je.

---

## 1. Tri rozdiely oproti promptu — nevymyslel som si ich, nedali sa splniť doslova

**`POST /api/integrations/discord/test` neexistuje a existovať nemôže.**
TIQR nemá HTTP server. Je to Tauri 2 desktop appka: „backend" sú Rust
funkcie volané cez IPC, nie endpointy. Takže to nie je route, ale príkaz
`test_discord_webhook`. Všetko ostatné zo špecifikácie — validácia cieľa,
cooldown, žiadne tajomstvá smerom von — platí rovnako, len to nesedí na
slovo „endpoint".

**Autentifikácia, oprávnenia a CSRF nie sú vynechané — nemajú tu zmysel.**
Toto je jednopoužívateľská lokálna appka. Niet sa voči komu autentifikovať,
niet roly, ktorú overiť, a niet cudzieho webu, ktorý by podstrčil
požiadavku. Keby som sem postavil vrstvu oprávnení, bola by to kulisa.
Hranica, ktorá tu **naozaj** existuje — frontend verzus backend — je
ustrážená a popísaná nižšie.

**Sales / Finance / Inventory nie sú kategórie notifikácií.** Appka ich
sleduje štyri a volajú sa inak: *UnpaidOrders, PendingSales,
MissingListingPrice, UpcomingEvents*. Sales, Finance a Inventory sú sekcie
appky. Postavil som ich presne tak, ako si ich pomenoval — tri samostatné
webhooky, tri kanály — ale **zatiaľ do nich nič automaticky nechodí.**
Dnes je to pripojenie a jeho test. Napojiť periodickú kontrolu na tieto tri
kanály je ďalší krok a tvoje rozhodnutie, ktorá udalosť má ísť kam.

---

## 2. Čo pribudlo

Discord v tejto appke **doteraz vôbec nebol.** Jediná zmienka v celom
projekte bol komentár vo `firebase.ts`, že Discord *prihlásenie* sa odkladá.
Takže toto nie je tlačidlo pridané k niečomu — je to celý kanál.

**Settings → Integrations → Discord.** Tam, kde si povedal, a je to správne
miesto: je to voliteľná externá služba, vedľa Google Sheets a Anthropic
kľúča. Tri webhooky, každý s tlačidlom **Test webhook**.

Nastavenia sa ukladajú na ten istý `NotificationConfig`, ktorý vlastní
Settings → Notifications, lebo Discord je tretí doručovací kanál k tým
istým upozorneniam (vedľa desktopu a ntfy). Ten rozpor — karta je v
Integrations, dáta v Notifications — je zámerný a **má jeden dôsledok, ktorý
som musel ošetriť**: uloženie z Discord karty prepisuje aj `desktopEnabled`
a `ntfyEnabled`. Preto si karta tesne pred zápisom načíta živý stav a tie dve
hodnoty prepošle nezmenené. Bez toho by ti uloženie Discordu potichu vyplo
desktop notifikácie.

---

## 3. Bezpečnosť — bod po bode, ako si ich napísal

**„Frontend posiela len identifikátor cieľa, nie URL."** Príkaz berie
`destination: String` a nič iné. `DiscordDestination::parse` prijme presne
`sales`, `finance`, `inventory` — všetko ostatné odmietne. Je na to test,
ktorý mu podsúva URL, `../sales`, `http://127.0.0.1:8080/` aj prázdny
reťazec. **Preto sa z tohto príkazu nedá urobiť HTTP proxy**: nemá ako
prijať adresu, na ktorú by poslal.

**„URL drž na backende."** Frontend ju nikdy nedostal, takže ju nemá ako
poslať späť. `NotificationStatus` nesie **len booleany** —
`discordSalesSet`, `discordFinanceSet`, `discordInventorySet`. Nie URL, nie
maskovanú URL, nie jej id.

**„Maskuj URL v Settings."** Bodky v UI nie sú skrytý reťazec — **za nimi
nič nie je.** Kreslia sa z tých booleanov. URL webhooku má token priamo v
ceste, takže URL *je* to heslo; jediné bezpečné maskovanie je neposlať ju.
Pole sa nikdy nepredvyplní a prázdne pole pri uložení znamená „nechaj, čo
je uložené" — rovnaká dohoda, akú už má ntfy topic.

**„Nikdy nevracaj URL ani token v odpovediach, logoch, chybách."**
- Modul **neloguje nič** — v celom `notifications.rs` nie je jediný
  `println!`, `log::` ani `tracing::`.
- Premenná s URL sa v celom Discord kóde objaví **na dvoch miestach**: v
  hlavičke funkcie a v `client.post(url)`. Nikde sa neformátuje do textu.
- Chybové hlášky z `validate_webhook_url` sú písané ručne tak, aby URL
  neobsahovali. Je na to test, ktorý skúša tri zlé URL a kontroluje, že
  v hláške nie je ani token, ani celá URL, ani ten cudzí host.

**„Nevracaj surové chybové telá z Discordu."** Telo odpovede sa **nikdy
nečíta.** Berie sa `.status()` a hlavička `retry-after`, nič viac. (Pre
porovnanie: staršia `send_ntfy` v tom istom súbore si telo do chyby dáva.
Nechal som ju tak — nie je to moja úloha a je to iný kanál.)

**„Validuj, že cieľ je jeden z nakonfigurovaných."** Dvakrát: meno cez
`parse`, a potom aj uložená URL cez `validate_webhook_url` proti štyrom
Discord hostom (`discord.com`, `discordapp.com`, `ptb.`, `canary.`) —
**pred každým odoslaním**, nielen pri ukladaní. Takže aj keby sa do configu
nejako dostala cudzia URL, nikdy sa na ňu nič nepošle. Testy skúšajú aj
`https://discord.com.evil.test/...` a `http://` namiesto `https://`.

**„Správa nesmie obsahovať zákaznícke dáta, prihlasovacie údaje, financie
ani sklad."** Testovacia správa sa skladá z troch argumentov a konštánt —
**nečíta z databázy nič.** Je na to test, ktorý v nej hľadá 24 zakázaných
podreťazcov (meny, „customer", „profit", „ticket", „token", „password"…)
a padne, ak by sa tam čokoľvek z nich dostalo.

**„Nevytváraj fiktívny predaj / transakciu / položku."** Príkaz nemá jediný
`INSERT` ani `UPDATE`. Jediný dotyk databázy je **čítanie** configu.

---

## 4. Tri stavy, nie dva — a prečo na tom záleží

Toto je jediná časť, kde som šiel nad rámec doslovného zadania, a dôvod je
priamo v tvojom prompte: *„Neposielaj test automaticky znova po nejasnom
timeoute, lebo Discord už mohol správu prijať."*

Aby sa to dalo dodržať, musí **existovať stav „neviem"**:

| stav | kedy | čo UI povie |
|---|---|---|
| `delivered` | **len** pri 2xx od Discordu | zelené „Delivered" |
| `failed` | Discord odpovedal a odmietol | červené „Failed (KÓD)" |
| `unknown` | nič sa nevrátilo (timeout, bez siete) | **oranžové „Not confirmed"** |

`unknown` sa **nefarbí ako chyba a nepočíta sa ako úspech.** Správa znie
doslova: *správa mohla, ale nemusela doraziť — pozri sa do kanála, než
testuješ znova.* A nič sa neposiela znova samo. To je presne to, čo si
žiadal, len to potrebovalo tretiu farbu.

Nič sa nikdy nevyhlási za doručené bez 2xx — je na to test, ktorý prejde
**všetkých 500 stavových kódov** a overí, že žiadny mimo 2xx neprejde ako
doručený.

Navyše: **cooldown 10 sekúnd na cieľ, na strane backendu.** Zakázané
tlačidlo zastaví dvojklik; toto zastaví zvyšok, lebo UI nie je hranica
dôvery.

---

## 5. Testy — 25, bez siete, bez novej závislosti

Žiadal si, aby sa Discord v testoch mockoval a aby testy nepotrebovali
skutočný webhook. Namiesto pridania mock-HTTP knižnice som **rozhodovaciu
tabuľku vyčlenil ako čistú funkciu** `interpret_discord_status(u16)`. Vďaka
tomu sa dá každá kategória chyby otestovať bez siete, bez mock servera a
**bez novej závislosti** — čo bola ďalšia tvoja podmienka.

Pokrýva: parsovanie cieľa (vrátane odmietnutia URL), validáciu URL (všetky
štyri hosty, cudzie hosty, `http://`, skrátené URL), neúnik tajomstva do
chýb, všetkých 500 stavových kódov, cooldown (vrátane hodín, ktoré idú
dozadu), obsah testovej správy a to, že `NotificationStatus` neobsahuje URL.

---

## 6. Čo našlo overovanie — a jedna vec, ktorú by som bez neho poslal zle

**Päť existujúcich testov by **nepreložilo**.** Pridanie poľa `discord` do
`NotificationConfig` a štyroch polí do `NotificationConfigInput` nechalo päť
starších test literálov v `notifications.rs` s neúplnou sadou polí → chyba
**E0063**, `cargo test` by spadol. Všetkých päť opravené.

Toto je **tretí raz**, čo presne táto trieda chyby zasiahla tento projekt.
Tentoraz ju našiel skript, nie CI.

**A jedna vec o samotných kontrolách, ktorú musím priznať.** Môj starý
kontrolór komentárov **prešiel súborom, ktorý obsahoval presne tú chybu
z 2.86.0**, na ktorú bol napísaný. Kontrola, ktorá prejde pokazeným
súborom, je horšia než žiadna — verí sa jej. Prepísal som ho na
štruktúrovaný skener a **pri tom sa ukázalo, že jedno z mojich vlastných
pravidiel bolo nesprávne**: tvrdil som, že `//` v zozname atribútov JSX
rozbije build. Nerozbije — je to legálne a je to v 12 súboroch, ktoré ti
roky fungujú. Pravidlo som zrušil, nie „opravil".

| Kontrola | Výsledok |
|---|---|
| Zátvorky, celý `src-tauri/src` | BALANCED |
| Zátvorky, všetkých 65 TS/TSX súborov | 65/65 OK |
| Komentárové pasce (A0/A2/A3) | 65 + 3 súbory — čisté |
| **Rust struct literály (E0063)** | **545 overených — OK** (3 preskočené, vypísané menovite) |
| Fantómové typy + registrácia príkazov | 243/243 príkazov zaregistrovaných |
| Render karty, tvoje farby, všetky tri stavy | pozreté |

Každá z tých kontrol **najprv padla na zámerne pokazenom súbore**, než som
jej uveril. Dve z nich mali pritom vlastnú chybu, ktorú to odhalilo:
kontrola struct literálov hlásila `-> Thing {` (návratový typ) ako literál
a počítala `>` v `if delta > 0` ako zátvorku.

**Nie je to kompilácia.** Node ani cargo na tomto Macu nie sú — CI je prvý
kompilátor, a dev server sa z toho istého dôvodu spustiť nedá. Render je
statická replika karty s tvojimi tokenmi, nie bežiaca appka.

---

## 7. Čo NIE JE hotové

- **Do troch kanálov zatiaľ nič automaticky nechodí.** Je to pripojenie a
  jeho test. Napojenie periodickej kontroly je ďalší krok — povedz, ktorá
  udalosť má ísť do ktorého kanála.
- **Pulls detail route + pohľad po osobách** (tvoje body 6 a 7 z 2.98.0) —
  stále odložené, stále jedna úloha, nie dve.

---

## 8. Verzia

2.98.0 → **2.99.0**, 9 výskytov v 7 súboroch plus `PROJECT_STATE/
CURRENT_STATE.md` a `CHANGELOG.md`.
