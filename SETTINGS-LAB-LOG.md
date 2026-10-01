# SETTINGS LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/QECgjpgSsP4JQnGGJ9ZPid
Sesterský denník: [DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md)

Tu si zapisujeme, čo sme pri settings stránke rozhodli a prečo. Ku
každej zmene jeden riadok. Na konci z toho spravíme finálnu verziu.

---

**01 — Vzhľad sa neprerokúva znova.**
Settings lab sa otvára presne v tom, čo si vybral na dashboarde: Noir,
TIQR v Syne, zvyšok v Instrument, normálna hustota, Slight rohy, pohyb
zapnutý. Tokeny sú skopírované 1:1 z dashboard labu, takže sa tie dve
stránky nemôžu rozísť. Doka to vie prepnúť, ale otvára sa na tvojom.

**02 — Desať návrhov je desať *štruktúr*, nie desať farebných schém.**
Health Board, Two Doors, One Page, Command Bar, Task Menu, Ledger,
Checklist, Data Sheet, Machine Map, In Place. Každý je iná odpoveď na
otázku „ako má byť stránka postavená", nie iný odtieň toho istého.

**03 — Žiadny návrh si nevymýšľa vlastné ovládanie.**
Všetkých desať pracuje s tou istou sadou ovládacích prvkov zo siedmich
naozajstných sekcií (Dáta, Zoznamy, Sheets, Upozornenia, Verzia, Účet,
Pomoc). Prepnutie návrhu mení **tvar stránky, nie čo stránka vie**.
Vyberáš si usporiadanie, nie funkcie.

**04 — Chrbtová kosť celej stránky je ten incident s €.**
Mesiace boli objednávky s menou uloženou ako `€` namiesto `EUR`, každý
súčet filtroval na `EUR`, a tie objednávky jednoducho neboli v žiadnom
čísle, na ktoré si sa pozeral. Nič v apke nemalo za úlohu si všimnúť
riadky, ktoré nič nepočíta. Sedem z desiatich navrhnutých funkcií
existuje kvôli tomuto.

**05 — Náhľad musí ukázať *chytenú chybu*, nie zelenú fajku.**
Každé „Try it" beží proti demo súboru so zasadenou chybou a vracia sa
červené alebo oranžové. Náhľad, ktorý sa vráti celý zelený, ťa naučí,
že funkcia nič nerobí.

**06 — Desať funkcií, zoradených podľa toho, čo najviac bolí.**
01 Missing From Totals · 02 List Cleanup · 03 Compare Machines ·
04 Test My Backup · 05 File Check · 06 My Rules · 07 Numbers Over Time ·
08 Import Preview · 09 Sheets Check · 10 What Changed.
Poradie je návrh, nie rozhodnutie — to je na tebe.

**07 — Nič odmietnuté sa nevrátilo.**
Price Checker, Ticket Center, Calendar, Inventory Intelligence, Market
Attention, Market vs Mine, Live Market Monitoring, Live Event
Intelligence — ani jedno, ani prezlečené pod iným menom. Žiadne
monitorovanie na pozadí, nič nebeží kým je apka zavretá, nič si
nedomýšľa dáta, ktoré si nezadal.

**08 — Import Preview ide dnu. Zvyšok settings ostáva.**
Prijaté. Ostatných jedenásť funkcií ostáva na stránke ako zoznam, ku
ktorému sa vrátime. Úloha pri settings teraz nie je prestavať ju, ale
spraviť ju **profesionálnejšou**.

**09 — Lab najprv nesedel so skutočnou apkou, teraz už sedí.**
Audit kódu našiel dve celé oblasti, ktoré prvý nástrel vynechal —
**synchronizácia medzi počítačmi** a **AI funkcie** — takže sú tam
teraz ako sekcie. Ukázal aj, že tri z desiatich funkcií sú už z
polovice postavené; pri každej je to napísané, nepredávam to dvakrát.

**10 — Jeden môj predpoklad bol zlý a je opravený.**
Myslel som si, že živá databáza ti leží v Drive priečinku a Drive ju
kopíruje spod rúk. Nie je to tak — tvoj cloud sync nahráva hotový
súbor cez Drive API, čo je správne. Riziko ostáva len vtedy, ak sa
`.db` náhodou ocitne v priečinku, ktorý sleduje niečo iné, a to dnes
nič nekontroluje. Funkcia 02 je preto prepísaná ako kontrola, nie ako
tvrdenie.

---

## Čo audit našiel v reálnom kóde

Toto nie je dizajn, toto sú **chyby v tom, čo dnes beží**. Zoradené
podľa toho, čo ťa môže stáť dáta alebo peniaze. Nič som neopravoval —
je to na tvoje rozhodnutie.

1. **Upozornenia sa pri prvom zapnutí neuložia.** Na čistej inštalácii
   `configured` počíta z rozpísaného formulára, nie z uloženého stavu.
   Keď klikneš prvý checkbox, formulár sa zbalí na zhrnutie, odznak
   ukáže „Enabled", tlačidlo Save zmizne — a **neuložilo sa nič**.

2. **Obnova staršej kópie z Drive nemá žiadne potvrdenie.**
   „Restore this" pri Drive verziách prepíše celú živú databázu a
   reštartuje apku. Jediné, čo to sprevádza, je popisok. Rovnako
   „Take theirs" a „Sync down" — celá databáza preč, bez dialógu.

3. **Text klame pri „Combine both".** Píše sa tam „nothing is replaced
   and nothing is deleted", ale zlúčenie **maže riadky, ktoré druhý
   počítač zmazal**. História to aj vypisuje ako „removed". Toto je
   najnebezpečnejšia veta v celých nastaveniach.

4. **Keď sa načítanie stavu nepodarí, vyzerá to ako platný stav.**
   Cloud sync si nastaví `null` a prepínač ukáže vypnuté bez chyby.
   Upozornenia ukážu prázdny formulár, ktorý by Save prepísal cez to,
   čo je uložené. Nikde nie je „nepodarilo sa načítať".

5. **Dve potvrdenia si protirečia.** Dialóg pri obnove hovorí „This
   cannot be undone", ale vedľa je napísané „undoable" a bezpečnostná
   kópia sa naozaj robí. Jedno z toho je zlé.

6. **ntfy tému sa nedá vymazať.** Prázdne pole znamená „nechaj ako je",
   takže sa dá len prepísať, nikdy zrušiť.

7. **„Profile updated." sa zobrazí aj keď sa nič neuložilo** — ak nie
   je prihlásený používateľ, funkcia ticho skončí a toast aj tak vyskočí.

8. **Verzia: „Latest version: Not checked yet"** vedľa „Last checked"
   s konkrétnym časom. Výsledok kontroly pri štarte sa zahadzuje.

9. **Drobnosti.** „Hide" a „Show fewer" pri restore points robia to
   isté. Vysvetlivky pri Sheets tlačidlách sú len v hover tooltipe,
   vrátane tej dôležitej, že „Push sales" vie vyprázdniť bunky.
   Staršie ako 20 bezpečnostných kópií leží na disku a z apky sa k nim
   nedostaneš.

---

## Otvorené

- Ktorý z desiatich návrhov.
- Ktoré funkcie ideme reálne stavať a v akom poradí.
- Či Sheets tabuľky vôbec ostávajú.
- Dobiehajú ešte dvaja agenti: rešerš (settings/backup/sync/integrita UX)
  a read-only audit skutočného `src/pages/Settings.tsx`. Keď dorazia,
  doplní sa sem, čo z toho zmenilo návrh.
