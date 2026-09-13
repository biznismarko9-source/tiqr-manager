# SYNC 2.18.0 — vylepšenie existujúceho Google Drive syncu

Bez nového Sync Centra, bez novej stránky, bez nového modulu, bez nového cloud
backendu. Model ostal presne ten istý: **lokálna SQLite ↔ tvoj Google Drive**.

---

## 1. Čo bolo na syncu opravené

**Preteky medzi dvoma syncmi.** Doteraz existovali len dve poistky a ani jedna
nevedela o tej druhej: `Layout.tsx` mal svoju pre časovač a Settings si zamykali
vlastné tlačidlá. Kým boli príkazy synchrónne, bežali za sebou a nikdy sa
nestretli — **2.17.0 ich presunulo mimo hlavné vlákno a tým sa to stalo reálne
možným**. Dva uploady naraz rozhodnú víťaza podľa toho, ktorý HTTP request
náhodou doskončí neskôr, a potom si zapíšu `version` súboru, ktorý ten druhý
medzitým prepísal. Teraz je jedna procesová poistka (`SyncGuard`), ktorú berie
**každý** vstupný bod — push, pull aj merge. Uvoľňuje sa pri zániku, teda aj pri
predčasnom návrate cez `?` a aj pri panике.

**Zbytočné uploady.** „Databáza bola zapísaná" a „databáza sa líši od kópie v
Drive" nie sú to isté. Spustenie migrácií pri štarte appky označí databázu za
zmenenú — takže **po každej aktualizácii sa nahrávalo niekoľko megabajtov, ktoré
Drive už mal, bajt po bajte**. Teraz si appka pamätá hash bajtov, ktoré naposledy
nahrala, a ak sa nelíšia, upload preskočí. Sťahovanie si ten istý hash zapíše, aby
sa čerstvo stiahnutá databáza hneď neposlala späť.

**Chýbajúci retry.** Žiadny neexistoval — jedno zlyhanie siete znamenalo čakať
päť minút na ďalší pokus.

---

## 2. Ako sa teraz rieši konflikt

Nič sa nemení ticho a nič sa nerozhoduje za teba:

| situácia | čo sa stane |
|---|---|
| zmenilo sa len tu | nahrá |
| zmenilo sa len tam | stiahne (pri štarte) |
| **zmenili sa obe strany** | **spojí** — záznam, čo má len jedna strana, sa skopíruje; čo majú obe, ostane tak, ako je tu |
| merge niečo **nevie** rozhodnúť | **STOP** — zapíše sa konflikt |

Merge ostal presne taký, aký bol v 2.16.0: iba vkladá, nikdy neprepisuje ani
nemaže, takže sa nedá nič stratiť. Čo **nevie** rozhodnúť — preskočené riadky a
`legacy-N` kolízie identity — teraz **nastaví príznak konfliktu, ktorý zmaže až
merge, ktorý prejde načisto**. Obyčajný upload ho už neprekryje zeleným
fajfkom, kým sa stroje o tých záznamoch stále nezhodujú.

Poistka proti prepísaniu (`remote_has_moved`) ostala nedotknutá: push, ktorý by
zmazal prácu druhého stroja, sa odmietne a jediná cesta ďalej je vedomé
„Overwrite".

---

## 3. Ako sa zabraňuje freezom a pretekom

**Freeze** vyriešilo 2.17.0 — 16 príkazov, čo chodia na sieť alebo hýbu celou
databázou, je `#[tauri::command(async)]`, takže nebežia na hlavnom vlákne.
2.18.0 na to nadväzuje: pauza medzi pokusmi o retry je bezpečná **iba preto**, že
tieto príkazy už nie sú na hlavnom vlákne. Na ňom by to bolo štvorsekundové
zamrznutie okna.

**Preteky** rieši `SyncGuard`. Prehľad toho, čo sa už nemôže stretnúť:

- dva uploady
- upload + download
- auto-sync + ručný sync
- merge + čokoľvek

Časovač pri obsadenej poistke odpovie `idle` (nie chybou) — spúšťa sa sám každých
päť minút, takže „teraz nič" je pravdivá odpoveď, nie zlyhanie.

Prístup k databáze naďalej serializuje ten istý mutex ako predtým, takže
atomickosť jednotlivých operácií sa nezmenila.

---

## 4. Offline

Appka funguje lokálne úplne normálne. Stav sa ukáže ako **Offline**, nič sa
nesťahuje ani nenahráva a **lokálne dáta sa nemenia**. Časovač skúsi znova o päť
minút — žiadne nekonečné opakovanie. Kontrola stavu pri otvorení panelu robí
jeden pokus bez retry, lebo čakať štyri sekundy na otvorenie Settings by bolo
horšie než úprimná odpoveď.

---

## 5. Retry

- **Opakuje sa:** prerušené spojenie, 429, 503 — dva ďalšie pokusy, pauza 1 s a 3 s.
- **Neopakuje sa:** 401, 403, 404, `invalid_grant` — to sú odpovede, nie výpadky.
  Opakovať ich znamená len dlhšie čakať na tú istú hlášku.
- **Nikdy nekonečne.** Po treťom pokuse to vzdá a povie prečo.

Pri chybe prihlásenia dostaneš krátku vetu a existujúci re-login flow v
Settings → Integrations ostal nezmenený.

---

## 6. Čo bolo optimalizované

- upload sa nespustí, keď sa bajty nelíšia od tých v Drive
- download sa nespustí, keď sa cloud nepohol (to platilo aj predtým)
- kontrola stavu robí jeden metadátový request, nie stiahnutie súboru
- hash sa počíta zo snapshotu, ktorý sa aj tak musí spraviť — takže pribudlo
  rádovo pár milisekúnd a ušetrí sa celý sieťový prenos

Priorita ostala **SAFE > FAST**: nič z toho nepreskakuje validáciu, zálohu ani
rollback.

---

## 7. Stav v UI

V karte, ktorá už existovala. Jedno slovo + posledný úspešný sync + jedna veta,
ak niečo zlyhalo:

`Synced` · `Syncing` · `Local changes` · `Cloud changes` · `Conflict` ·
`Offline` · `Failed`

Slovo určuje backend (`summarize_state`), aby si panel nikdy nemohol myslieť
niečo iné než časovač.

---

## 8. Zmenené súbory

| súbor | čo |
|---|---|
| `src-tauri/src/commands/cloud_sync.rs` | SyncGuard, retry, content hash, summarize_state, záznam chyby, push/pull rozdelené na príkaz + telo |
| `src-tauri/src/commands/cloud_merge.rs` | SyncGuard, retry na sieťových volaniach, príznak konfliktu, rozdelenie na príkaz + telo |
| `src/lib/types.ts` | `state`, `localChanges`, `lastError` v `CloudSyncStatus` |
| `src/pages/Settings.tsx` | riadok so stavom v existujúcej sync karte |
| `CHANGELOG.md`, `PROJECT_STATE/*` | dokumentácia |

**Nezmenené:** refund/resell, `batch_id`, peniaze, Orders, Tickets, Sales,
Listings, Finance, Fulfillment, Attention, Calendar, Price Checker, AI. Žiadna
zmena schémy, žiadna migrácia (ďalšia je stále 028), žiadna nová závislosť.

---

## 9. Testy

15 nových v `cloud_sync.rs` (spolu 34):

- poistka odmietne druhý sync a po uvoľnení funguje znova
- auth chyba sa neopakuje / výpadok siete áno
- retry sa zastaví po limite
- hash: rovnaké bajty = rovnaký, zmenený bajt aj dĺžka = iný
- všetkých osem stavov vrátane „konflikt prežije offline"
- skrátenie chybovej hlášky

Existujúce a naďalej platné: rozhodovacia tabuľka auto-syncu (offline, prázdny
Drive, jedna strana, obe strany), merge (kolízia kódov, kolízia mena platformy,
`legacy-N` identita, otrávené počítadlo, preskočený riadok), backup/restore
(validácia, záloha pred restore, rollback).

---

## 10. Limity — čo som NEsľúbil

- **Merge stále neprenáša úpravu ani zmazanie.** Vloženie má jeden správny
  výsledok, úprava dva. Mazanie potrebuje náhrobky a vlastnú migráciu.
- **`legacy-N` kolízie sa iba hlásia, neopravujú.** Ktorá `legacy-7` je ktorá,
  je tvoje rozhodnutie.
- **Hash je FNV-1a, nie kryptografický.** Nechráni pred podvrhnutou databázou,
  iba odpovedá na „sú to tie isté bajty". Zlyháva v bezpečnom smere.
- **Kompilátor som tu nespustil** — na tomto Macu nie je Rust ani Node. Logika
  je overená staticky a algoritmy proti skutočnej SQLite; `cargo test --lib`,
  `npx tsc -b` a `npm run build` prejdú až v CI.
