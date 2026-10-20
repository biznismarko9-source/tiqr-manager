# TIQR Manager 2.12.0 — Cloud Sync

Čo si napíšeš na Macu, uvidíš na Windows. Presne to zadanie.

---

## ⚠️ Dve veci na začiatok

1. **Build som nespustil** — nie je tu Node ani Rust. Pribudlo 5 Rust testov, nebežali.
2. **Ani jedno volanie na Google Drive nikdy nebežalo.** Tvary requestov som napísal podľa Drive v3 API dokumentácie. Ak sync nefunguje, toto je prvá vec, ktorú treba pozrieť.

**A jedna vec, ktorú musíš urobiť po update:**

### Budeš sa musieť znova prihlásiť cez Google

Sync potrebuje prístup k tvojmu Drive. Doterajší sign-in scope ho neobsahoval, takže tvoj existujúci token Drive prístup neudelí. Settings → Integrations → prihlás sa znova a povoľ Drive.

---

## 1. Ako to funguje

Jeden súbor `tiqr-manager-sync.sqlite3` **v tvojom vlastnom Google Drive**.

```
Sync up    →  nahrá databázu z tohto počítača
Sync down  →  stiahne a obnoví databázu z druhého počítača
```

Typický deň: dopíšeš niečo na Macu → **Sync up** → prídeš k Windows → **Sync down** → máš to tam.

Nájdeš to v **Settings → Data**, nad Backup/Restore — je to tá istá vec (presúvanie databázy), len automatická.

---

## 2. Čo to zámerne NEROBÍ

**Nezlučuje jednotlivé zmeny.** Synchronizuje celú databázu naraz.

Prečo — konkrétne, nie výhovorka:

- Každý primárny kľúč v appke je `INTEGER AUTOINCREMENT` **per počítač**. Na Macu vytvoríš objednávku `id=5`, na Windows inú objednávku tiež `id=5`. Pri zlučovaní sú to dva rôzne záznamy s tým istým kľúčom.
- `insert_order_with_tickets` rozdeľuje náklad objednávky na tickety **na presný cent**. Zlúčenie dvoch verzií ten súčet rozbije.
- `refund_sale_impl` je jednosmerná atomická transakcia. Ak na jednom stroji refunduješ a na druhom predáš ten istý ticket, vznikne stav, ktorý appka považuje za nemožný — a žiadny algoritmus nevie, ktorá pravda je správna.

Skutočné zlučovanie = migrácia všetkých ~30 tabuliek na UUID + politika riešenia konfliktov. To sú mesiace, nie jeden release. Povedal som ti to pred stavaním a rozhodol si sa pre toto.

---

## 3. Prečo nemôžeš prísť o dáta potichu

**Poistka proti prepísaniu.** Pri každom nahratí si appka zapamätá `version` súboru v Drive. Pred ďalším nahratím ju znova overí:

```
Ak sa vzdialená verzia zmenila odkedy si naposledy synchronizoval
  → nahratie sa ODMIETNE
  → appka ti povie, že druhý počítač má novšie dáta
  → prepísať sa dá, ale iba ako samostatné vedomé rozhodnutie
```

Dialóg je červený a hovorí presne, čo sa stane.

**Sync down je bezpečný z inej strany.** Ide cez existujúci `restore_database_impl`, takže dedí:
- validáciu stiahnutého súboru (nezmysel sa neaplikuje)
- **automatický safety backup** tvojich súčasných dát *pred* čímkoľvek
- automatický rollback, ak čokoľvek zlyhá

Cestu k tomu safety backupu ti appka po sync down ukáže.

---

## 4. Čo som znovupoužil (a čo je naozaj nové)

Toto bolo podstatne lacnejšie, než sa zdalo, lebo appka už mala skoro všetko:

| Vec | Stav |
|---|---|
| Google OAuth + refresh tokeny | ✅ existovalo (Sheets sync, v produkcii) |
| Drive API zapnuté v GCP projekte | ✅ existovalo |
| SQLite Online Backup API | ✅ existovalo (`backup.rs`) |
| Validácia + safety backup + rollback pri restore | ✅ existovalo |
| **Drive sync logika** | 🆕 `commands/cloud_sync.rs` |
| **`snapshot_db_to`** | 🆕 vyextrahované z `create_safety_backup`, aby obe cesty zdieľali jednu implementáciu |
| **`drive.file` v sign-in scope** | 🆕 (preto to opätovné prihlásenie) |

Žiadny server, žiadna nová služba, žiadna nová dependency, žiadny mesačný poplatok.

`drive.file` je **najužší scope**, ktorý to zvládne: appka vidí iba súbory, ktoré sama vytvorila. Do zvyšku tvojho Drive nevidí.

---

## 5. Nič nebeží automaticky

Žiadny timer, žiadny sync pri štarte, žiadny sync pri zápise. Sync je vypnutý, kým ho nezapneš, a každá synchronizácia je klik.

Keď je vypnutý alebo si offline, appka funguje presne ako doteraz — **local-first zostáva pravda**.

---

## 6. Zmenené súbory

**Backend (5):**
- `commands/cloud_sync.rs` — **nový**, 4 príkazy + 5 testov
- `commands/backup.rs` — vyextrahované `snapshot_db_to`
- `google_oauth.rs` — `drive.file` v `OAUTH_SCOPE`
- `commands/mod.rs`, `lib.rs` — registrácia

**Frontend (3):**
- `pages/Settings.tsx` — sync karta + overwrite dialóg
- `lib/types.ts`, `lib/api.ts` — typy a bindingy

**DB: 0 zmien.** Žiadna migrácia (ďalšia je stále 027), žiadna tabuľka. Stav syncu si drží `app_settings` — existujúca key/value tabuľka.

---

## 7. Testy

5 nových Rust testov na logiku poistky:

| Test | Čo overuje |
|---|---|
| `a_machine_that_has_never_synced_treats_any_remote_file_as_newer` | prvý sync na druhom stroji neprepíše |
| `an_unchanged_remote_version_is_not_a_conflict` | bežný sync prejde |
| `a_changed_remote_version_is_a_conflict` | druhý stroj písal → odmietnuť |
| `a_missing_remote_version_never_blocks_an_explicit_push` | poistka nesmie sync znemožniť |
| `sync_is_off_until_it_is_explicitly_turned_on` | žiadny sync bez súhlasu |

**Nebežali.**

Staticky overené: zátvorky vo všetkých 8 zmenených súboroch, všetky `#[test]` naviazané na `fn`, TS importy sa rozlišujú, žiadne miešané typy v poli (tá chyba, čo zhodila 2.9.0).

Jednu vec som pritom zachytil: `CloudSyncStatus` som do Settings pridal bez `type` prefixu — pri `isolatedModules: true` by to bundler skúsil importovať ako runtime hodnotu z type-only súboru. Opravené.

---

## 8. Limity, ktoré musíš vedieť

- **Celá databáza naraz.** Ak píšeš na oboch strojoch bez synchronizácie medzi tým, jedna strana vyhrá. Nezmizne to ticho — appka ťa zastaví — ale zlúčiť to nevie.
- **Nie je to real-time.** Zmena sa objaví, až keď na druhom stroji klikneš Sync down.
- **Databáza opúšťa tvoj počítač.** Ide do tvojho Google Drive. Keďže cez Sheets sync tam už časť dát posielaš, nie je to nový vendor — ale je to zmena oproti „fully local".
- **Sync down je deštruktívny.** Nahradí lokálne dáta. Safety backup sa berie vždy, ale je to nahradenie, nie zlúčenie.

---

## 9. Ako to prvýkrát rozbehnúť

1. Nainštaluj 2.12.0 na **oba** počítače
2. Na oboch: Settings → Integrations → **prihlás sa znova cez Google**, povoľ Drive
3. Na tom počítači, ktorý má **správne dáta**: Settings → Data → zapni **cloud sync** → **Sync up**
4. Na druhom počítači: zapni sync → **Sync down**
5. Appka sa reštartuje a máš tam tie isté dáta

**Krok 3 rob na tom stroji, ktorý má dáta, ktoré chceš zachovať.** Druhý stroj svoje dáta pri sync down nahradí (so safety backupom).

---

## Čo sa nezmenilo

Žiadna business logika: Orders · Tickets · Sales · Listings · Finance · Fulfillment · Attention · Calendar · Price Checker · refund/resell · `batch_id` · money/integer cents. Žiadna schéma, žiadna migrácia, žiadna nová dependency, žiadny server.
