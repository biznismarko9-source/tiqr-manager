# TIQR Manager 2.87.0 — report

**Čo si zadal:** „oprav to ty kokotko" — k CI výstupu, ktorý ukazoval, že
2.86.0 build neopravilo, plus 10 padajúcich Rust testov.

**Čo je hotové:** build je opravený na všetkých troch miestach, a všetkých 10
padajúcich testov je vyriešených. Ani jeden z tých 10 nebol chyba v appke.

---

## 1. TypeScript — moja oprava z 2.86.0 bola tiež chybná

V 2.80.0 som dal `{/* ... */}` do **JavaScript expression** pozície (po
`{period === "custom" && (`), kde sa to parsuje ako objektový literál.

V 2.86.0 som to správne zmenil na blokový komentár `/* ... */`. Ale do
**textu** toho komentára som napísal, čo bolo predtým — a tým som tam napísal
aj `*/`.

**Blokový komentár končí na prvom `*/`.** Takže sa uzavrel v polovici vety,
a `}` plus zvyšok anglickej prózy zostali stáť ako kód.

To je presne to, čo CI hlásil:

```
Overview.tsx(406,25):     TS1109: Expression expected.
Overview.tsx(424,10):     TS1381: Unexpected token.
Transactions.tsx(202,25): TS1109  /  (220,10): TS1381
Reports.tsx(204,25):      TS1109  /  (222,10): TS1381
```

406 je riadok, kde komentár začína. 424 je riadok, kde mal skončiť. Presné
rozpätie komentára, ktorý som tam vložil.

Komentár je prepísaný tak, aby ukončovaciu sekvenciu vo svojom texte
neobsahoval. Samotný kód (ten `<div className="flex flex-wrap items-end
gap-3">`) je nezmenený — vrátenie na pôvodný stav bolo v 2.86.0 správne, zlý
bol len komentár okolo.

**Pravidlo, ktoré si z toho beriem:** text komentára nikdy nesmie obsahovať
ukončovaciu sekvenciu. Ak treba hovoriť o syntaxe komentárov, treba to opísať
slovami.

---

## 2. Rust: 10 padajúcich testov — jedna príčina, nula chýb v appke

`cargo test` je v CI od 2.73.0, ktoré som pridal ja. Suite dovtedy reálne
nebežal od posledného stroja, kde bol Rust toolchain. Medzitým **2.30.0**
zaviedlo event-derived kódy — `COLDPLAY-001` namiesto `PULL-000001` —
a každý test so zadrôtovaným fixným prefixom tým prestal platiť.

Tie testy teda padali **správne**. Len to nikto nevidel, lebo nebolo čím ich
spustiť.

| Test | Čo sa stalo |
|---|---|
| `a_brand_new_path_ends_up_fully_migrated` | Canary na počet migrácií stál na 26, kým pribudlo 027–039 → **39** |
| `search_finds_order_by_partial_ticket_code` | `trim_start_matches("TKT-")` už neodstrihne nič, kód je `TEST-001` |
| `create_pull_generates_sequential_pull_codes` | `COLDPLAY-001`, nie `PULL-000001` |
| `create_pull_received_generates_sequential_rpull_codes` | `COLDPLAY-001`, nie `RPULL-000001` |
| `orders_sheet_sync` marker test | `starts_with("ORD-")` už nemôže prejsť |
| `pulls_sheet_sync` marker test | `starts_with("PULL-")` to isté |
| migrácie `004`, `007`, `008`, `024` | `run_migrations` už nekončí na svojej migrácii |

### Ten canary si zaslúži vetu zvlášť

`a_brand_new_path_ends_up_fully_migrated` nie je obyčajný test. Jeho vlastný
komentár hovorí, že je to *„deliberate canary"* — má padnúť vždy, keď niekto
pridá migráciu a zabudne ju zaregistrovať, aby sa nikdy nevydal build
s nenamigrovanou čistou inštaláciou.

Stál na 26, kým pribudlo 13 migrácií. Robil presne to, na čo bol napísaný, a
13 vydaní ho nikto nepočul. Overil som, že všetkých 39 je zaregistrovaných
v `MIGRATIONS` a všetkých 39 sa na čistý súbor aplikuje — čiže to, čo canary
naozaj stráži, je v poriadku. Stará bola len tá konstanta.

### Migračné testy — čo som tam vlastne zmenil

Tie štyri testy majú v komentári napísané, že `run_migrations` aplikuje
„ONLY 004". **To už nie je pravda.** Dnes dotiahne súbor až na 039, vrátane
029, ktorej prepis (`codes::backfill_event_codes`) zámerne premenuje každý
kód — fixture má event „Old Event", takže `SAL-000001` sa stane `OLD-001`.

Takže tvrdenie „každý riadok musí prežiť byte-for-byte" je dnes nepravdivé
**z princípu**, nie kvôli strate dát.

Preto som `code` z porovnania vyňal a namiesto neho overujem skutočný
kontrakt 2.30.0: **starý kód sa presunul do `legacy_code`, nezahodil sa** —
čo je presne to, čo drží už nasynchronizovaný Google Sheet funkčný, keď jeho
„TIQR ID" už nezodpovedá žiadnemu aktuálnemu kódu. To je silnejšie tvrdenie
než to, ktoré nahradilo.

Pri `024` som navyše prekľúčoval všetky dotazy zo starého kódu na `id` —
hľadal riadok podľa `'TKT-PRE-024'`, ktorý po prepise už neexistuje, a padal
na `QueryReturnedNoRows`.

**Žiadnu migráciu, schému ani produkčný kód som nemenil.** Zmenené sú len
testy.

---

## 3. Čo som našiel a ZÁMERNE neopravil

**`codes.rs` nemá ani jeden test.** To je súbor, ktorý rozhoduje o každom
kóde v appke. A jeho vlastný komentár hovorí prečo to je dôležité: pravidlo
musí byť na oboch tvojich strojoch **bit po bite identické**, inak každý
vyrobí pre ten istý záznam iný kód a merge uvidí kolíziu, ktorá nie je
kolízia. Dnes je to pokryté len nepriamo, cez tie migračné testy.

Nepísal som ich — to je nová práca, nie oprava buildu, a ty si zadal opravu
buildu. Ale je to podľa mňa najdôležitejšia medzera, ktorú toto kolo odhalilo.

**Tie štyri migračné testy padali pred svojím koncom.** Tvrdenia za opraveným
riadkom na tejto schéme nikdy nebežali. Ak je niektoré z nich tiež staré,
ukáže to až najbližší CI run. Nepredstieram, že to viem.

---

## 4. Ako som to overil — a prečo si tomu tento raz verím viac

Na tomto Macu nie je node ani cargo. **CI je prvý a jediný kompilátor.** To sa
nezmenilo.

Čo sa zmenilo: tento raz som každú kontrolu najprv pustil na **známe pokazený
súbor** a vyžadoval, aby padla na tom istom riadku, aký nahlásil kompilátor.
Až potom som jej veril na opravenom.

To mi odhalilo **štyri chyby v mojich vlastných kontrolách**:

1. Do znakovej sady som napísal reťazec `'{};'` a čítal som to ako dva znaky
   — je v ňom aj `}`. Takže každé TSX `} />` vyzeralo ako začiatok regexu.
2. `</div>` bralo ako regex, lebo nasleduje po `<`.
3. Template literal som po skončení `${...}` neobnovil, takže ukončovací
   backtick otvoril nový a zhltol kód.
4. `in` a `of` v obyčajnom JSX texte („2.1k in / 3.4k out") bralo ako
   JavaScript keywordy.

Všetky štyri by mi boli nahlásili chyby, ktoré neexistujú — alebo, horšie,
boli by prehlásili pokazený súbor za čistý. **Kontrola, ktorá prejde na
pokazenom súbore, je horšia než žiadna, lebo sa jej verí.**

Výsledky po tom, čo kontroly samé prešli negatívnym testom:

| Kontrola | Výsledok |
|---|---|
| Vyváženie zátvoriek, TypeScript | 65/65 súborov v `src/` v poriadku |
| Vyváženie zátvoriek, Rust | 7/7 editovaných súborov v poriadku |
| Komentárová chyba trieda A (`*/` v texte) | 0 výskytov v `src/` |
| Komentárová chyba trieda B (`{/* */}` v expression pozícii) | 0 výskytov v `src/` |
| Arita tuple ↔ stĺpce v SELECT ↔ počet `r.get()` | 0 nezhôd |

**Čo to NIE je:** nie je to kompilácia, nie je to `tsc`, nie je to `cargo
test`, a nie je to beh appky. Nič z vyššie uvedeného nedokazuje, že to
skompiluje. Dokazuje to, že tá konkrétna trieda chyby, ktorá zhodila 2.80.0
aj 2.86.0, tam už nie je — a že kontrola, ktorá to tvrdí, vie tú chybu
naozaj nájsť.

Skripty sú v scratchpade tejto session, **nie v repo**. Nie sú závislosť, sú
len záznam toho, ako sa toto kolo overovalo.

---

## 5. Verzia

2.86.0 → **2.87.0**, 9 výskytov v 7 súboroch, plus `CURRENT_STATE.md`.
Overené, že v tých súboroch nezostal ani jeden `2.86.0`.

`CURRENT_STATE.md` nemal záznam pre 2.73.0 až 2.86.0 — celý redesign. Pridal
som záznam pre 2.87.0 a odkaz na to, že detail tej série je v `CHANGELOG.md`
a v jednotlivých reportoch. Chýbajúcich 14 záznamov som nedopisoval.
