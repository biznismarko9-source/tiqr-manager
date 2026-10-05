# TIQR Manager 2.68.0 - automatický sync sa už pýta

**Dátum:** 4.10.2026
**Zadanie:** *"ten sync nefunguje spravne, stracam udaje"* -> *"oprav to aby sa to uz nestalo"*

---

## 1. Čo sa stalo

1.10.2026 o **16:47** automatický sync stiahol z Google Drive **staršiu**
kópiu databázy a prepísal ňou celú tvoju lokálnu databázu.

Preč bolo:

| | |
|---|---|
| ESPAA-002 | ESPAÑA - INGLATERRA, 30.09., 1 ks, 50,00 € |
| ESPAA-003 | ESPAÑA - INGLATERRA, 30.09., 3 ks, 150,00 € |
| ESPAA-004 | ESPAÑA - INGLATERRA, 30.09., 2 ks, 80,00 € |
| 6 lístkov | všetky `available` |
| OASIS-001 | 30.09., 320,00 €, pending |
| OASIS-002 | 30.09., 320,00 €, pending |

V Settings -> Data boli **štyri** body obnovy s názvom "Before Sync Down".
Nestalo sa to prvýkrát.

## 2. Ako viem, že to bol prepis súboru a nie mazanie

Dve veci, obe overené priamo v databázach:

1. Tabuľka `deleted_rows` sa cez tú stratu **nezmenila**. Keby si tie riadky
   zmazal ty alebo merge, pribudli by tam tombstony. Nepribudli.
2. Živá databáza bola **striktná podmnožina** zálohy z 16:47 - nič navyše,
   len menej.

To je podpis `cloud_sync_pull`: celý súbor sa vymení za iný.

## 3. Prečo si to appka dovolila

V zálohe `pre-restore-20261001-164651520.sqlite3` je zapísané:

```
cloud_sync_local_dirty      false
cloud_sync_last_sync_at     2026-09-30T13:23
```

Tie objednávky a predaje vznikli **30.09. o 14:36** - hodinu PO poslednom
syncu. Čiže boli neodoslané. Príznak `local_dirty`, ktorý má presne toto
strážiť, napriek tomu tvrdil `false`.

Z toho `decide_auto` v `cloud_sync.rs` vyrobil:

```rust
(false, true) => (Pull, "The other machine has newer data and this one has nothing unsent.")
```

a `Layout.tsx` ten Pull **vykonal na časovači, bez otázky**.

## 4. Čo som zmenil

### Pull a Merge sa pýtajú

`src/components/Layout.tsx` - päťminútový tick už nevykonáva ani jedno
z nich. Prácu zaparkuje a otvorí potvrdzovacie okno.

- **Pull** - červené okno: *"Replace this computer's data with Google Drive's
  copy?"* Hovorí rovno, že sa všetko nahradí, že sa predtým uloží záloha
  a že sa appka reštartuje.
- **Merge** - žlté okno: *"Combine what's on both computers?"* Hovorí, že sa
  pridajú záznamy z druhého počítača **a že sa zmaže to, čo si zmazal tam**.

**Push ostáva automatický.** Iba odosiela - lokálne nemôže stratiť nič -
a vlastnú poistku proti prepísaniu druhého počítača už má (to okno
"Overwrite your other computer's data?").

Telo oboch operácií je **nezmenené**. Len je o jeden `async () =>` hlbšie
a spustí sa až keď klikneš. Žiadna nová cesta k tvojim dátam nevznikla.

### "Nie" sa pamätá

Keď dáš Cancel, otázka sa **neopakuje každých 5 minút**. Ostane banner hore
a spustíš si to v Settings -> Data, keď chceš ty.

### História syncu už neklame

Keď sa sync len **spýtal**, zapíše `"asked"`. Predtým by zapísal `"pull"` -
čiže stiahnutie, ktoré sa nestalo. Pri chybe, kde celý problém bol, že sync
tvrdil niečo iné, než robil, je to podstatné.

### Opravený nepravdivý komentár

Komentár z 2.16.0 tvrdil, že merge *"nothing is replaced and nothing is
deleted, so there is no question left to put to marko"*. Druhá polovica
nebola nikdy pravda - `totalDeleted` priamo pod ním ráta riadky, ktoré merge
zmaže podľa tombstonov z druhého stroja. To bolo zdôvodnenie, prečo merge
beží bez opýtania. Zdôvodnenie padlo, okno je späť.

## 5. Čo som vedome NEOPRAVIL

**Prečo bol `local_dirty` nepravdivý, ostáva otvorené.** Zapísal som to do
`PROJECT_STATE/KNOWN_BUGS.md` aj s tromi kandidátmi.

Správna oprava je neveriť príznaku a porovnať obsah - lenže `content_hash`
beží nad celým snapshotom databázy a `cloud_sync_auto` je dnes zámerne
lacný read-only príkaz, ktorý beží každých 5 minút. To je skutočná zmena
návrhu, nie malá úprava, a nemám tu kompilátor.

Dialóg robí to zlyhanie **nedeštruktívnym bez ohľadu na to, prečo je príznak
zlý**. To je dôvod, prečo som začal ním.

**Ďalší krok, ktorý odporúčam:** v `decide_auto` zmeniť vetvu
`(false, true)` z `Pull` na `Merge`. Merge nie je nikdy deštruktívnejší než
Pull - aplikuje tie isté tombstony, ale **nechá riadky, ktoré druhá strana
nikdy nevidela**. Čiže je to lepšie predvolené správanie aj keď je príznak
správny. Treba k tomu dopísať unit testy na konci `cloud_sync.rs`. Nerobil
som to teraz, lebo je to druhá nekompilovaná zmena v jadre syncu navrch
urgentnej opravy a dnes už nič navyše nechráni - Pull sa aj tak nespustí
bez teba.

## 6. Čo je otestované a čo nie

**Overené:**

- Všetkých 9 výskytov verzie v 7 súboroch je na 2.68.0, nikde nezostalo 2.67.0.
- Každý nový identifikátor (`pendingSync`, `pendingRunRef`, `pendingSyncRef`,
  `syncDeclinedRef`, ...) má deklaráciu; `ConfirmDialog` je naozaj exportovaný
  z `ui.tsx` (riadok 1371) a `Spinner` z importu nevypadol.
- Zátvorky: rozdiel oproti netknutej zálohe súboru je 0/0/0 pre `{}`, `()`
  aj `[]`. Vložené bloky sú samy o sebe vyvážené.
- Dáta: po tvojej obnove sú ESPAA-002/003/004, OASIS-001/002 aj 7 lístkov
  späť (49 objednávok, 119 lístkov, 39 predajov).

**Neoverené - povedané rovno:**

- **Nič z tohto nie je skompilované.** Na tomto Macu nie je node, npm ani
  cargo. Prvý kompilátor je CI pri builde. Platí to aj pre 2.65.0, 2.66.0
  a 2.67.0.
- Okno som nevidel bežať. Kontroloval som ho čítaním, nie klikaním.
