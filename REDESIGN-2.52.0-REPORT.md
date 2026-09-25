# TIQR Manager 2.52.0 — Workspace

Toto je tá vec, čo si pýtal: *„miesto na ktoré sa mozem spolahnut, nieco kde
si viem zapisat napr aky kod a co som komu predal, aky je jeho nick, ake info
treba mat ulozene atd, proste vsetkko co vies vyuzit v beznej praxy, mozno
nejake plany, ulozit si ucty, prehlad, najst vsetky jednoducho."*

Celé to stojí na štyroch krokoch: **zapíš → usporiadaj → nájdi → použi**.

---

## Kde to je

**Vlastná položka v paneli — `Workspace`.** Nie je to záložka vo Finance.

V 2.51.1 to pod Finance bolo, lebo si to tak vtedy chcel, keď to boli len
tabuľky. Teraz to je aj o kupcoch, účtoch, plánoch a úlohách — a nick kupca
nie je finančný záznam. Takže **Finance je späť na svojich štyroch
záložkách** (Overview · Transactions · Accounts · Reports) a Workspace stojí
sám.

---

## Čo si tam vieš vytvoriť

| Typ | Na čo |
|---|---|
| **Note** | obyčajný text — čo ti napadne, čo si potrebuješ pamätať |
| **Record** | poznámka s **vlastnými políčkami** — Nick, Kód, Cena, Kontakt, čokoľvek si pomenuješ |
| **Task** | úloha s **termínom** a stavom Open / Done |
| **Table** | tabuľka s vlastnými stĺpcami — to sú presne tie hárky z 2.51.0 |

### A toto je na tom to hlavné

**Nemusíš vopred vedieť, čo z toho to bude.**

Napíšeš rýchlu poznámku „Jano chce 4 kódy na Oasis". O dva dni si uvedomíš,
že to je vlastne úloha s termínom — otvoríš ju, v editore prepneš `Kind` na
**Task**, dáš dátum. Alebo na **Record** a dopíšeš mu nick a kód.

**Nič sa nestratí a nič nevytváraš odznova.** Je to stále tá istá položka —
tie isté tagy, ten istý dátum vytvorenia, tá istá história. Políčka, checklist
aj termín ostávajú uložené, len ich ten typ, ktorý ich nepotrebuje, neukazuje.

Preto to nie sú tri oddelené zoznamy, ale jeden. Keby to boli tri, každá také
prepnutie by znamenalo zmazať a vytvoriť nanovo — a na druhom počítači by sa
to po synchronizácii tvárilo ako úplne iná vec.

---

## Rýchly zápis

Hore je **jeden riadok a Enter**. To je celé.

Nič sa neotvára, nič nevypĺňaš, nič nevyberáš. Keď píšeš viac riadkov, prvý sa
stane názvom. Detaily doplníš, keď na to budeš mať — alebo nikdy, ak netreba.

To bolo zámerné: ak zapísať si niečo trvá desať sekúnd a tri kliky, tak si to
nezapíšeš.

---

## Nájsť to

Vyhľadávanie ide **cez všetko naraz** — názvy, text, políčka (aj ich hodnoty),
tagy a názvy tabuliek — a výsledky ti **poukladá podľa typu**, aby si videl,
či to, čo hľadáš, je poznámka, record, úloha alebo tabuľka.

Pri každom výsledku vidíš **kúsok textu okolo toho, čo si hľadal**, nie len
názov. Takže hneď vieš, či je to ono, bez otvárania.

**Archivované sa nehľadá.** Keď niečo odložíš, tak je to odložené.

### Aj ostatné spôsoby, ako sa k veciam dostať

- **Tagy** — klikneš na tag a máš všetko s ním
- **Kategórie** — General, Buyers, Codes, Accounts, Tasks, Plans, Important, Other
- **Pripnutie** — čo používaš stále, drž hore
- **Upcoming** — najbližšie termíny, s „Today" a „Tomorrow" napísaným slovom
- **Naposledy upravené** — čo si riešil naposledy, je prvé
- **Archív** — nemažeš, len odkladáš; a vieš sa k tomu vrátiť

---

## Heslá — a poviem to na rovinu

Keď políčko pomenuješ ako heslo — **password, heslo, pass, pin, secret, token,
api key, 2fa, seed** — jeho hodnota sa automaticky **zmení na bodky**. V
editore aj vo výsledkoch hľadania. Odkryje sa až keď klikneš **Show**.

**Ale: nie je to zašifrované.**

Databáza je ten istý obyčajný súbor ako Sales, Orders a Finance. Kto sa
dostane k tvojmu počítaču, dostane sa k tomu aj tak. To skrývanie je len o
tom, **aby ti heslo nesvietilo na obrazovke, keď si ho nepýtal** — napríklad
keď niekomu ukazuješ appku alebo si len niečo hľadáš.

**Workspace nie je správca hesiel a nebudem sa tváriť, že je.** Ak by si chcel
naozajstné šifrovanie, to je samostatná robota a musíš si ju vypýtať.

---

## Tabuľky — tvoje staré hárky, nezmenené

Tie hárky z 2.51.0 sú tam všetky. **Tie isté dáta, tie isté stĺpce, to isté
automatické ukladanie bunky**, keď z nej klikneš preč.

Zmenilo sa len to, že sa k nim dostaneš cez Workspace, a že tlačidlo
**← Back to Workspace** ťa vráti späť do prehľadu.

Databázové tabuľky som **zámerne nepremenoval**, aj keď sa to v appke teraz
volá inak. Tvoj druhý počítač si pamätá, čo si kde zmazal, **podľa ich mien** —
keby som ich premenoval, po synchronizácii by sa ti niektoré zmazané riadky
vrátili.

---

## Synchronizácia

Workspace sa synchronizuje **presne tak ako všetko ostatné** — cez ten istý
Google Drive, tým istým spôsobom, v tých istých momentoch.

**Do synchronizácie som nesiahol.** Nepridal som žiadne nové spúšťače, nič som
neprerábal. Len som zaregistroval novú tabuľku tam, kde musí byť, aby sa pri
zlučovaní dvoch počítačov nestratila.

---

## Čo som nechytal

Dashboard, Events, Inventory, Sales, Pulls, Finance, prihlásenie, nastavenia,
téma, navigácia. Nič z toho som neupravoval a nič z toho sa nemá správať inak.

Databáza sa len **rozšírila** — migrácia 032 pridáva jednu novú tabuľku a päť
nových stĺpcov k hárkom. Nič existujúce sa nemaže ani neprepisuje.

---

## Čo tam ZATIAĽ nie je — aby si to nehľadal

Toto som vedome nechal na neskôr, radšej ti to napíšem, než aby si to hľadal:

- **Typy stĺpcov v tabuľkách** (číslo, dátum, áno/nie) — **každá bunka je
  stále obyčajný text**. Miesto v databáze na to už je pripravené, ale appka
  to zatiaľ nečíta, takže ti nič nekontroluje.
- **Klávesové skratky** — všetko sa robí myšou (okrem Enter pri rýchlom zápise).
- **Triedenie a filtrovanie vnútri tabuľky** — hľadanie ide cez celý
  Workspace, ale v otvorenej tabuľke nevieš klikať na stĺpec a triediť.
- **Odkazy sa nepremenia na klikacie** — keď napíšeš `https://...`, ostane to
  text.
- **Presúvanie stĺpcov** — stĺpec vieš pridať, premenovať a zmazať, ale nie
  potiahnuť na iné miesto.
- **Move / Convert v menu** — prepínať typ vieš cez `Kind` v editore, ale nie
  pravým klikom priamo na karte.

Ak ti niektorá z týchto chýba v praxi, povedz a dorobím ju.

---

## Čo som overil a čo nie

**Overené:**

- migrácia 032 som pustil na naozajstnej databáze postavenej zo všetkých 32
  migrácií — prešla, uid sa priradí sám, mazanie nechá stopu pre
  synchronizáciu, **hárky z 2.51.0 sa dajú stále čítať**
- synchronizačné zapojenie novej tabuľky (všetky tri potrebné časti)
- Finance je naozaj späť na štyroch záložkách

**Neoverené na mojej strane:** nemám tu Node ani Rust, takže **appku
nezostavím** — to spraví až tvoj build. Rovnako som neklikal cez hotovú appku.
Keby čokoľvek nesadlo, pošli screenshot a opravím to.
