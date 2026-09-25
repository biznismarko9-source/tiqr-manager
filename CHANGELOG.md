# Changelog

Short, append-only entries, newest at the top - one entry per completed
task under the TIQR development protocol (see step 8). This is not a
replacement for the detailed `REDESIGN-X.Y.Z-REPORT.md` / `*-REPORT.md`
files at the repo root (one per release, written for marko, in Slovak) -
those still get written for real releases. This log exists so a future
session can see recent activity at a glance without opening any of them.

(2026-09-01: this file merges two copies that grew independently in
different sessions after 2.0.80 - one bootstrapped at 2.0.80 with entries
back to 2.0.75, the other bootstrapped at 2.1.9 with entries from 2.1.9
onward. Versions 2.0.81-2.1.8 - Finance module, Price Checker auto-check
iterations - fall in the gap between the two bootstraps and are not
backfilled here, consistent with this file's own existing policy below;
read the matching `REDESIGN-X.Y.Z-REPORT.md`/`*-REPORT.md` for any of
those directly.)

## 2.52.0 - Workspace

Nová vlastná položka v paneli: **Workspace**. Miesto, kam si zapíšeš všetko,
čo potrebuješ mať poruke — komu si čo predal, aký kód, aký nick, heslá k
účtom, plány, úlohy s termínom — a hlavne to celé vieš zas nájsť.

Finance sa vracia na svoje štyri záložky. Workspace je samostatná sekcia,
nie je to súčasť Finance.

### Štyri veci, ktoré si vieš vytvoriť

- **Note** — obyčajná poznámka, text.
- **Record** — poznámka s vlastnými políčkami (Nick, Kód, Cena, čokoľvek).
- **Task** — úloha s termínom a stavom (Open / Done).
- **Table** — tabuľka s vlastnými stĺpcami (to sú tie hárky z 2.51.0,
  nezmenené, aj s tým, čo si do nich napísal).

**Nemusíš vopred vedieť, čo z toho to bude.** Napíšeš rýchlu poznámku a
kedykoľvek ju v editore prepneš na Record alebo Task — nič sa nestratí, je to
stále tá istá položka.

### Rýchly zápis

Hore je jeden riadok: napíšeš a dáš Enter. Hotovo. Nič sa neotvára, nič
nevypĺňaš. Detaily doplníš, keď budeš chcieť.

### Nájsť to

Vyhľadávanie prehľadá **názvy, text, políčka, tagy aj názvy tabuliek** naraz
a výsledky poukladá podľa typu. Vidíš kúsok textu okolo toho, čo si hľadal.

Ďalej: **tagy**, **kategórie**, **pripnutie** hore, **archív** (nemažeš, len
odložíš), **Upcoming** s najbližšími termínmi a prehľad naposledy upraveného.

### Heslá

Ak políčko pomenuješ ako heslo (password, heslo, pin, token, 2fa…), jeho
hodnota sa **automaticky skryje na bodky** — v editore aj vo vyhľadávaní — a
odkryje sa až keď klikneš Show.

**Dôležité a hovorím to na rovinu:** toto je len o tom, aby ti heslo
nesvietilo na obrazovke. **Nie je to šifrované.** Databáza je ten istý
obyčajný súbor ako Sales a Finance. Workspace nie je správca hesiel.

### Čo sa nezmenilo

Dashboard, Events, Inventory, Sales, Pulls, Finance, prihlásenie,
synchronizácia, nastavenia, téma, navigácia — nič z toho som nechytal.
Synchronizácia funguje presne tak ako doteraz, len navyše prenáša aj
Workspace.

## 2.51.1 - Notes je teraz pod Finance

Presunuté z vlastnej položky v paneli na **piatu záložku vo Finance**:
Overview · Transactions · Accounts · Reports · **Notes**.

**S dátami sa nestalo nič** — tie isté tabuľky, tá istá migrácia, to isté
synchronizovanie. Čo si stihol napísať v 2.51.0, tam ostáva.

## 2.51.0 - Notes: miesto na všetko dôležité

Nová položka v paneli: **Notes**. Hárky, ktorým si **sám pomenuješ stĺpce** —
ako v Google Sheets, ale vnútri appky a synchronizované medzi oboma počítačmi.

### Ako to funguje

Vytvoríš si hárok a píšeš riadky. Stĺpce si kedykoľvek **pridáš, premenuješ
alebo zmažeš**. Každá bunka sa **uloží sama**, hneď ako z nej klikneš preč —
žiadne tlačidlo Uložiť, takže nikdy nie je nič „ešte neuložené".

### Aby to nebol prázdny papier

Nový hárok vieš začať zo **šablóny**:

- **Buyers** — Nick · Ticket code · Event · Paid · Contact · Note
- **Accounts** — Platform · Account · Email · Note
- **Plans** — What · By when · Status · Note
- **Blank** — jeden stĺpec, pomenuj si ho

Stĺpce sú len začiatok, hárok je potom tvoj.

### Nájdeš v tom všetko

Hore je **jedno hľadanie cez všetky hárky naraz**. Napíšeš nick, kód lístka
alebo kus mena — a vypíše ti to riadok, **v ktorom hárku** je a **v ktorom
stĺpci** sa to našlo. Klikneš a si tam.

### Synchronizuje sa

Zápisky idú medzi Macom a Windowsom rovnako ako objednávky — vrátane zlučovania,
keď si na oboch počítačoch napísal niečo iné, aj mazania.

## 2.50.1 - sync sa už nespúšťa pri každom kliknutí na okno

Moja regresia z 2.48.1. Vtedy som pridal, že sa syncuje aj keď sa vrátiš na
okno — aby bolo prepnutie medzi počítačmi okamžité. Fungovalo to až príliš
dobre: **každé alt-tabnutie spustilo sync** a zakaždým to o sebe dalo vedieť
v hlavičke.

Preč. Zostáva **jeden sync pri spustení appky** a potom ticho na pozadí každých
5 minút.

Ten časovač som nechal naschvál — keby som zrušil aj jeho, vrátila by sa presne
tá vec, na ktorú si sa sťažoval predtým (že sa počítače nespoja, keď necháš
appku otvorenú). Na rozdiel od toho focusu je ale **neviditeľný**, kým naozaj
nie je čo preniesť.

## 2.50.0 - euro sa už nekonvertuje na euro + preferovaná mena

### Tá chyba

V databáze bol uložený **symbol `€`**, nie kód `EUR`. A kontrola „je to už
v eurách?" porovnávala doslova s textom `"EUR"` — `"€"` sa mu nerovná, takže
objednávka zadaná v eurách išla konvertovať **z eur na eurá**. A služba na
kurzy, celkom správne, žiadnu menu `€` nepozná. Odtiaľ to 404.

Teraz appka rozpoznáva symboly ako kódy: **€ = EUR, $ = USD, £ = GBP**, aj
`Kč`, `zł`, `Ft`, `lei`, `лв`, `₺`. Čo je už v cieľovej mene, sa **nikdy
neponúkne na konverziu**.

`kr` som naschvál nechal tak — je to švédska, nórska **aj** dánska koruna.
Tipovať by znamenalo prepočítať ti peniaze zlým kurzom.

### Preferovaná mena

**Settings → Lookups → Preferred currency**: na výber **EUR, USD, GBP**.

Všetko „Convert to…" v celej appke sa riadi podľa toho — Dashboard, obe
tlačidlá pri objednávke, Sales aj Finance. Keď to prepneš, **popisky sa zmenia
naraz všade** a nemôžu sa rozísť s tým, čo konverzia naozaj spraví.

Default je EUR, takže ak to nikdy neotvoríš, appka sa správa presne ako doteraz.

## 2.49.2 - povolenie sa dá dať jedným klikom priamo z hlášky

Preveril som ešte dve veci, ktoré by boli naozajstné chyby v kóde — a **obe sú
čisté**: appka si pri obnove tokenu nepýta užší rozsah, a to druhé prihlásenie
(„Continue with Google") ti ten prvý token neprepisuje.

**Zostáva to, čo sa z mojej strany opraviť nedá.** Aké povolenia token má, sa
rozhodlo v momente, keď si ho schválil — a **žiadny kód nevie do tokenu pridať
povolenie, ktoré mu nikdy nikto nedal.** To je pravidlo Googlu, nie chyba appky.
Takže úplne „bez zmien" to nejde a nebudem ti tvrdiť opak.

**Čo som spravil:** zrušil som to hľadanie. V tej červenej hláške je teraz
tlačidlo **„Allow Google Drive access"** — klikneš, otvorí sa Google, schváliš,
a **sync sa rozbehne hneď**. Žiadne chodenie do Settings.

Spolu s 2.49.1 je to uzavreté: ak by si pri tom schvaľovaní niektoré políčko
odškrtol, appka to **odmietne uložiť** a povie ti to — takže sa to nemôže
potichu zopakovať.

## 2.49.1 - to 403 bolo chýbajúce povolenie

Tá chyba, čo si poslal, je jednoznačná: **`insufficientPermissions`**. Nie je to
vypnuté API ani expirovaný token — **tvoj prihlasovací token nemá povolenie na
Drive**.

Ako sa to stane: na tej Google obrazovke má **každé povolenie vlastné
zaškrtávacie políčko**. Keď jedno odškrtneš, Google ti aj tak vráti úplne
platný token — len bez toho povolenia.

**A appka sa nikdy nepozrela, čo Google naozaj povolil.** Uložila taký token ako
úspešné prihlásenie a každé ďalšie volanie na Drive padlo na 403. Bez stopy,
bez vysvetlenia.

Teraz sa to kontroluje: ak pri prihlásení chýba Drive (alebo Sheets),
**prihlásenie rovno zlyhá a povie ktoré** — namiesto toho, aby sa uložilo
a lámalo sa to až potom.

**A tá hláška ťa posielala na zlé miesto.** Začínala tým, že „Drive API nie je
zapnuté v Google Cloud projekte" — čo nebol tvoj problém. Teraz sa číta
odpoveď od Googlu a podľa nej sa povie buď „prihlás sa znova a nechaj všetko
zaškrtnuté", alebo „zapni API, tu je odkaz".

**Čo s tým máš spraviť ty:** Settings → Integrations → prihlásiť sa Googlom
znova a na tej obrazovke **nechať zaškrtnuté všetky políčka**.

## 2.49.0 - dátum sa dá písať, mena sa vyberá

### Dátum: píš alebo klikni, oboje

Ono to nebolo pokazené — **ono sa doň nikdy nedalo písať**. To políčko bolo
v skutočnosti tlačidlo, ktoré len otváralo kalendár.

Teraz je to **normálne políčko**. Klikneš, píšeš číslice a bodky si doplní samo:
`21092026` → `21.09.2026`. Kalendár zostáva — je to tá ikonka vpravo v políčku.

Nezoberie dátum, ktorý neexistuje: **31.02.2026** je osem správnych číslic a aj
tak to nie je deň. Priestupný rok sedí — 29.02.2024 áno, 29.02.2026 nie.

Rozpísané `21.0` sa do formulára nedostane. Keď odklikneš preč bez dokončenia,
vráti sa posledná platná hodnota.

### Mena sa vyberá zo zoznamu

Pri **order, sale aj pull** je mena rozbaľovačka s tými istými 13 menami, aké
ponúka zvyšok appky. Keby si mal niekde menu mimo zoznamu, zostane ti —
appka ti ju ticho neprepíše na EUR.

## 2.48.1 - a toto bola tá chyba

Našiel som ju. **Zlučovanie a sťahovanie smelo bežať len pri jednom jedinom
tiku — hneď po otvorení appky.** A práve vtedy ešte zvyčajne nie je pripravené
prihlásenie do Googlu, takže ten jediný pokus vrátil „vypnuté" a **minul sa,
hoci nikdy žiadnu šancu nedostal**. Každý ďalší tik už právo zlučovať nemal —
len ukázal pruh a čakal. Donekonečna.

Preto sa ti tie dva počítače nikdy nespojili.

**Teraz to smie každý tik.** Jediné, čo to odloží, je keď máš **otvorené okno
alebo rozpísané políčko** — zlúčenie totiž do databázy iba pridáva riadky a tá
nie je v ohrození, ale obnovenie stránky po ňom by ti zmazalo rozrobené. Vtedy
ti to napíše a dokončí sa hneď, ako dopíšeš.

**Plus: syncuje sa aj vtedy, keď sa na okno vrátiš.** Predtým si po prepnutí
z druhého počítača čakal aj päť minút — a práve to je ten pocit „nie je to
automatické".

**Nezacyklí sa to:** po zlúčení sa uloží verzia a databáza zostane „špinavá",
takže ďalší krok je nahranie spojených dát hore a potom pokoj.

## 2.48.0 - autosync konečne povie, čo robí

Prešiel som celú cestu autosyncu a **v samotnej logike som chybu nenašiel** —
príkazy sú zaregistrované, názvy sedia, rozhodovacia tabuľka je správna,
príznak „mám neodoslané zmeny" sa nastavuje pri každom zápise a zámok sa
nemôže zaseknúť.

**Čo bolo naozaj pokazené:** autosync **prehĺtal každú chybu**. V kóde bol
prázdny `catch {}`. Takže počítač, ktorému sa upload každých 5 minút odmietal,
vyzeral úplne rovnako ako počítač, ktorý nemá čo poslať — **na oboch stranách,
koľko chcelo dní**.

Teraz:

- **Červený pruh hore** s konkrétnou chybou, keď autosync zlyhá.
- **Každý pokus sa zapíše** — aj ten, čo nemal čo robiť — a v **Settings → Data**
  vidíš posledných 8 pokusov aj s časom a dôvodom, zvlášť na každom počítači.

Ten existujúci riadok „Last synced" ti to povedať nevedel: počíta aj ručné
synchronizácie, takže mohol hlásiť „pred hodinou", kým automatika medzitým
zlyhala dvanásťkrát.

## 2.47.5 - štyri slovenské slová, ktoré sken nevidel

Mal si pravdu. Môj sken v 2.47.3 hľadal slovenčinu **podľa diakritiky** — takže
slovo bez dĺžňov a mäkčeňov mu prešlo popod ruky. Štyri také tam boli:

| Slovo | Kde |
|---|---|
| `nie` | prepínač Pull v **New Order** |
| `Kupec` | popis poľa v **New Sale** |
| `Ks` (2×) | hlavička stĺpca v zozname **Pulls** |
| `e.g. Doprava` | placeholder v Settings |

Teraz je to `no`, `Buyer`, `Qty` a `e.g. Transport`.

**Ako som to našiel poriadne:** nie lepším zoznamom slov, ale tak, že som
vypísal **všetkých 733 textov**, ktoré appka zobrazuje, a tých 399 krátkych
(nadpisy, tlačidlá, popisy polí) som prečítal.

## 2.47.4 - Profit karta už nie je fialová

Na light mode svietila karta Profit levanduľovo medzi štyrmi bielymi — vyzerala
ako označená, a zelené číslo na nej sa s tou farbou bilo.

Boli to **dve chyby v jednom riadku**:

- `border-brand-300` **nikdy nič nenakreslilo** — karta má nastavené
  `border: 0`, takže farba rámika nemala čo zafarbiť. Jediné, čo bolo vidieť,
  bola tá výplň.
- `brand-50` je sýta levanduľa. Ako jediná zafarbená dlaždica v rade bielych
  pôsobila ako výber, nie ako dôležitý údaj.

Teraz je tam **neutrálny tenký krúžok** a číslo zostáva väčšie (24 px oproti
19 px) — nájdeš ho hneď, ale nebije sa to.

Ostatné fialové miesta v appke (lišty výberu, čipy, oznamy) som nechal — tie
majú byť fialové, sú to akcie.

## 2.47.3 - celá appka je po anglicky

Vypĺňovače boli od 2.45.0 po slovensky, lebo vznikli z toho slovenského
preview. Teraz je všetko po anglicky.

**127 reťazcov v 8 súboroch** — štyri vypĺňovače, spodná lišta a ikonky
v `ui.tsx`, dve hlášky v AI paneli, jeden placeholder v Settings a jeden
príklad mena vo Finance.

Našiel som ich tak, že som preskenoval **každý reťazec a každý text v celom
`src/`** na slovenskú diakritiku a slová — a po oprave som ten istý sken
spustil znova. Vrátil nulu.

**Čo zostalo po slovensky: komentáre v kóde.** Tie citujú tvoje vlastné
zadania, prečo je niečo tak, ako je — a nikto ich v appke nevidí.

## 2.47.2 - vypĺňovač je širší a stĺpce sa roztiahli

Okno malo **napevno 1152 px** bez ohľadu na to, aký veľký máš monitor — preto
Typ ukazoval len „—" a Sektor s Radom boli užšie než ich vlastné nadpisy.

Teraz je **až 1560 px**, respektíve 94 % šírky okna, keď máš menšie. Rozpočet
na stĺpce narástol z 1112 na **1520 px** a najviac dostali tie, čo boli
najstlačenejšie:

| Stĺpec | Predtým | Teraz |
|---|---|---|
| Typ | 110 px | **150 px** |
| Sektor | 92 px | **130 px** |
| Rad | 64 px | **90 px** |
| Sedadlá | 108 px | **150 px** |
| Platforma | 130 px | **190 px** |
| Poznámka | 126 px | **230 px** |

Platí to na všetkých štyroch — order, pull, sale aj event.

## 2.47.1 - dátum, Ks a rozpoznávanie mien

### Dátum nebol pokazený, bol orezaný

Kalendár sa otváral ako `absolute` vnútri tabuľky, ktorá sa posúva — a taký
kontajner absolútne umiestnené okno **oreže**. Takže sa otvoril do neviditeľna.

Teraz je `fixed` a počíta si vlastnú pozíciu. **Opravené naraz pri order, pull
aj event** — je to jeden komponent, bola to jedna chyba.

### Ks: šípky preč, políčko väčšie

`type="number"` kreslí šípky **dovnútra** políčka a pri 56 px sadli rovno na
číslo — presne ako na tvojej fotke. Teraz je to obyčajné políčko, píšeš doň len
číslice a je širšie (order 56 → 72 px, pull 50 → 70 px). Šírku som vzal
z Poznámky, takže sa stále všetko zmestí.

### AI: meno eventu a platformy sa konečne trafí

Porovnávalo sa **presne znak po znaku**. Takže „Karpatské Chalupy 2026"
z fotky nenašlo event „Karpatské Chalupy" a „TICKETPORTAL.SK" nenašlo
„Ticketportal" — pole ostalo prázdne a nikto ti nepovedal prečo.

Teraz sa ignoruje diakritika, veľkosť písmen aj medzery navyše, a skúsi sa aj
začiatok a obsiahnutie. **Keď sedia dve možnosti, nevyberie ani jednu** —
tipovať medzi dvoma eventmi je horšie než prázdne pole.

### Dátum nákupu z fotky sa už nezahadzuje

AI ho z účtenky čítala vždy, formulár ho ignoroval a dal dnešný. Už nie.

## 2.47.0 - fotka vyplní viac objednávok naraz, rohy sú ostrejšie

Vybral si si **mriežku** — riadková tabuľka zostáva na všetkých štyroch.

### Fotka teraz vytvorí toľko riadkov, koľko skupín na nej je

Doteraz appka zo screenshotu prečítala všetky bloky sedadiel, ale **dala ti
vybrať jeden** a o zvyšku napísala, že si druhú objednávku máš spraviť ručne.
Preč.

Teraz **každá skupina = jeden riadok**. A čo je jedna objednávka a čo dve,
rozhodne **to isté pravidlo, aké platí, keď riadky píšeš sám**: musí sedieť
sektor, rad, cena, mena, typ, platforma aj pull — **a sedadlá musia ísť tesne
za sebou**. Inak vzniknú samostatné objednávky.

Takže screenshot s troma blokmi:

| Na fotke | Riadky | Objednávky |
|---|---|---|
| A/12 14–15 · A/12 16–18 · VIP/1 22 (rovnaká cena) | 3 | **2** — A/12 sa spojí na 14–18, VIP zvlášť |
| A/12 14–15 @201 · A/12 16–18 **@185** | 2 | **2** — iná cena sa nezlučuje |
| A/12 14–15 · A/12 **19** | 2 | **2** — sedadlá nie sú vedľa seba |
| A/12 · **B/3** | 2 | **2** — iný sektor vždy zvlášť |

**Prázdny formulár sa nahradí**, rozpísaný sa **doplní** — čo si už napísal,
sa nestratí. A fotka bez detailu lístkov ti riadok **nezmaže**, len doplní to,
čo z nej vyčítala.

### Fotka je teraz na všetkých štyroch

Sales bol jediný bez nej. Riadky v predaji sú **skutočné lístky**, takže tie zo
screenshotu vyčarovať nejde — ale vyplní sa dátum, platforma, kupec, stav
platby, a **cena aj poplatok sa predvyplnia do riadkov, ktoré ešte žiadnu
nemajú**. Cenu, ktorú si napísal ty, neprepíše.

### Ostrejšie rohy

Z **13 / 20 / 24 px** na **6 / 8 / 10 px**. Jedna zmena v jednom súbore, takže
to chytí celá appka vrátane Finance — netreba sa nikde inde hrabať.

Guličky, avatary a pilulky (`rounded-full`) som nechal okrúhle. Bodka s rohom
nie je ostrejší design, to je chyba.

## 2.46.1 - riadky sa zmestia a Ks je zase číslo

Mal si pravdu dvakrát.

### Ks už nie je zhasnuté políčko

Keď napíšeš sedadlá, počet z nich vychádza — ale v 2.46.0 som to ukazoval v
**read-only políčku**, čo vyzerá ako niečo, čo ťa odmieta. Teraz je tam
**samotné číslo**, zarovnané doprava. Keď sedadlá nenapíšeš, políčko na
písanie počtu je tam normálne ďalej.

### Naozaj sa to nezmestilo — zmeral som to

Okno má ~1112 px využiteľnej šírky. Stĺpce mali:

| Formulár | Predtým | Teraz |
|---|---|---|
| Inventory | 1296 px | 1112 px |
| **Pulls** | **1426 px** | 1112 px |
| Events | 1206 px | 1112 px |
| Sales | 966 px | 1054 px |

Pull pretekal o **314 px**. Teraz sedí každý.

### Ostrejší design

- **Hustejšie bunky** — prepnuté na kompaktný krok, ktorý appka už má
  (`px-2 py-2` namiesto `px-3 py-2.5`). Nič nového som nevymýšľal, len som
  použil to, čo tu bolo. Tým sa aj získala tá šírka.
- **Čísla a ich hlavičky sú zarovnané rovnako** — doprava. Číslo vpravo pod
  nadpisom vľavo je klasická známka tabuľky, ktorú nikto poriadne nenastavil.
- Užšie číslo riadku (28 px) a užší stĺpec s ikonkami (64 px), drobnejšie
  ikonky.

## 2.46.0 - vyplňovače vedia presne, čo je zle, a graf má os

### Chyba svieti na políčku, nie vo vete dole

Doteraz ti formulár povedal prvú chybu ako vetu a zvyšok zamlčal. Teraz
**každé zlé políčko zčervenie** — tým istým červeným krúžkom, aký má zvyšok
appky — a dole je napísané, koľko vecí treba opraviť a kde začať.

**Kedy to zasvieti:** ak si niečo napísal zle (cena „201,0x"), hneď. Ak niečo
len chýba, až keď klikneš Vytvoriť — prázdny formulár ešte nie je chybný.

### Ďalšie drobnosti, ktoré to robia rýchlejším

- **Čísla riadkov** — chyba vie povedať „riadok 3" a ty ho nájdeš bez počítania.
- **Hlavička zostáva** pri rolovaní dlhého zoznamu.
- **Duplikovať riadok** — šesť miest toho istého turné alebo štyri lístky, čo
  sa líšia sedadlom, sú rýchlejšie skopírované ako prepísané.
- **Kurzor ide do nového riadku**, keď ho pridáš.
- **⌘↵ / Ctrl+↵ vytvorí** odkiaľkoľvek z formulára.

### Ks a sedadlá si už neodporujú

Keď napíšeš sedadlá, počet kusov **z nich vychádza** — políčko to teraz ukáže
a nedá sa prepísať. Predtým tam ticho ostávala jednotka, ktorú appka aj tak
ignorovala.

### Finance: graf už nie je bodka

Default je „This month" a ten mal presne **jeden mesačný stĺpec** — takže graf
vyzeral prázdny. Teraz sa veľkosť kroku riadi podľa obdobia: **do 92 dní po
dňoch, nad to po mesiacoch**, rovnako ako to robí graf na Dashboarde. „This
month" je tým pádom ~30 bodov, teda skutočná krivka.

**Čo som zámerne neurobil:** nerozťahujem graf mimo zvoleného obdobia, len aby
bola čiara dlhšia. Mesiace mimo obdobia by sa kreslili ako nula, hoci v nich
reálne peniaze boli — to by bol graf, ktorý klame. Prázdne dni vnútri obdobia
sú naozajstné nuly.

## 2.45.0 - jeden vyplňovač všade, a otvára sa NAD zoznamom

Tvoja požiadavka mala dve časti a obe sú hotové.

### 1. Rovnaké riadky na všetkom

**Events, Inventory, Sales aj Pulls** majú teraz ten istý vyplňovač: tabuľka
riadkov, pod ňou tlačidlo „pridaj ďalší", dole jedna lišta, ktorá hovorí, čo
vznikne, a vedľa nej Zrušiť/Vytvoriť.

- **Inventory** — 11 polí, gulička na pull, dátum nákupu automaticky. Čo
  nesedí sektorom, radom, cenou, menou, typom, platformou či pullom — alebo
  nemá sedadlá tesne za sebou — je vlastná objednávka.
- **Pulls** — jeden riadok = jeden pull. Event sa píše (nie je to tvoja
  objednávka) a ďalší riadok si ho aj s dátumom prevezme.
- **Sales** — riadky sú skutočné lístky, takže „ďalší riadok" znamená
  „prihoď lístky z ďalšej objednávky". Zisk sa ráta hneď v riadku.
- **Events** — jeden riadok = jeden event, takže turné napíšeš na jeden raz.
  Ďalší riadok prevezme kategóriu a krajinu.

### 2. Žiadne vlastné okno — otvára sa nad zoznamom

New Order už nie je stránka. Všetky štyri sú okno **nad** svojím zoznamom a
pod ním vidíš rozmazané ostatné riadky — presne ako to robili Sales a Pulls
predtým. `/orders/new` zmizlo; stará linka ťa pošle na Inventory.

Editovanie sa nemenilo: event upravuješ na jeho detaile, pull v jeho okne,
oba pôvodné formuláre sú nedotknuté.

### 3. Finance: Income vs Expenses je graf z Dashboardu

Dvojité stĺpce sú preč. Je tam **ten istý graf ako na Dashboard → Overview** —
tá istá plynulá krivka, to isté zvýraznenie pri prejdení myšou, tie isté
prepínače. Tri: **Income · Expenses · Net**. Čísla sú tie isté, čo kreslili
stĺpce; veľké číslo nad grafom je súčet za obdobie z kariet nad ním, aby si
nikdy neukazovali dve rôzne sumy.

### Prečo 2.45.0 a nie 2.44.0

Zip s názvom `tiqr-manager-2.44.0.zip` už máš v Downloads a má iný obsah
(finance migrácia, ktorú sme opustili). Aby ti v Downloads neležali dva rôzne
súbory s tým istým názvom, táto verzia ide o krok ďalej.

## 2.43.0 - bočný panel je plochý

Skupina **Tickets** z bočného panela odišla. **Events, Inventory, Sales a
Pulls sú vždy vidieť** — nič sa neotvára a nezatvára, nič sa neskrýva.

Panel má teraz šesť riadkov v poradí, v akom práca beží: Dashboard · Events ·
Inventory · Sales · Pulls · Finance. Settings zostáva dole, kde bol.

**Čo to upratalo popri tom:** s rámčekom zmizol aj rozbaľovací stav, výpočet
„som niekde v tejto skupine" a nepoužitý typ riadku-nadpisu, ktorý nemal ani
jednu položku od 2.36.0. Vykresľovanie panela je jedna vetva namiesto troch.

Otázka, či zvýrazniť skupinu spolu s položkou (2.29.4 → 2.41.0), tým padá
sama: skupina už nie je.

## 2.42.0 - nová objednávka je stránka a riadky

Presne tá kombinácia, ktorú si vybral v preview: **celá stránka**, tabuľkové
**riadky**, pridávanie tlačidlom, **event raz hore**, mena/pull/poznámka
viditeľné.

### Jeden riadok = jedno miesto

Vyberieš event a pridávaš riadky. **Jedenásť polí, nič viac** — ks, typ,
sektor, rad, sedadlá, platforma, cena za kus, mena, pull, poznámka.

**Pull je gulička.** Zhasnutá = nie. Rozsvietená = pullnuté a vedľa vyskočí
políčko na meno. Meno prežije vypnutie.

### Čo nesedí, je vlastná objednávka

Dva riadky sa zlúčia do jednej objednávky **len vtedy**, keď sa zhoduje
všetko — sektor, rad, cena, mena, typ, platforma, pull — **a sedadlá idú tesne
za sebou**. `14-15` a `16` je jedna objednávka 14-16; `14-15` a `19` sú dve.
Cena sa porovnáva ako **peniaze**, nie ako text, takže „201,00" a „201.00" je
tá istá cena.

Tlačidlo dole rovno povie, koľko objednávok vznikne.

### Dátum nákupu sa nepýta

Stavia sa dnešným dátumom, opečiatkovaný raz pri otvorení stránky. Prepísať sa
dá naďalej na detaile objednávky.

### Čo sa už nepýta

**Poplatky a „ostatné náklady"** formulár nezbiera — cena, ktorú napíšeš, je
celý náklad na kus. **Stav platby** zostáva „paid", ako bol predvolený od
2.0.70. **Poplatok za pull** sa zadáva tam, kde vždy patril — na detaile
objednávky.

### Kam to vedie

`/orders/new`. Tlačidlá **New Order** v Inventory aj **New order for this
event** v detaile eventu smerujú sem; starý odkaz so `state` sa presmeruje,
takže nič nekončí naslepo.

Stará modálka `OrderFormModal` zostáva v súbore **nepoužitá, jednu verziu**,
aby sa dalo porovnať. Povedz a zmizne.

## 2.41.0 - jedna vec svieti, o tabuľku menej, panel preč

Štyri veci z tvojho zadania, ktoré sú rozhodnuté a nezávisia na ničom
otvorenom. Všetko frontend — žiadna migrácia, žiadny Rust.

### Svieti len položka

V ľavom paneli sa už nezvýrazňuje skupina **Tickets** spolu s položkou.
Svieti jedna vec: tá, na ktorej si.

**Toto je návrat k tomu, čo bolo pred 2.29.4** — vtedy si písal opak
(„ked mam nieco vybrate v tickets tak tickets niesu oznacene"). Napísané v
`PROTECTED_AREAS.md`, aby to o pol roka nikto „neopravil" späť podľa starého
komentára v kóde.

### Detail eventu: len lístky

Tabuľka **Orders** z detailu eventu odchádza. Objednávka je spôsob, akým si
zásobu kúpil; keď otvoríš event, zaujíma ťa, **čo na ňom máš** — a každý
lístok aj tak nesie kód svojej objednávky.

Tlačidlo **New order for this event** sa presunulo na hlavičku Tickets, takže
cesta dnu je tam, kam sa aj tak pozeráš. Objednávky sa nikam nestratili —
zoznam `/orders` je nedotknutý a karta Listings ich stále dostáva.

### Pravý panel preč

„What this will create" zmizol z **novej objednávky** aj z **nového eventu**.
Prepisoval polia, ktoré si práve vyplnil, o stĺpec vedľa — a kvôli tomu bol
každý formulár dvojstĺpcový. Formulár je zase jeden stĺpec. Komponent
`PreviewPanel` aj `preview` prop na modále sú zmazané, nie len nepoužité.

### Event ukazuje aj dátum

V **novej objednávke** bol dátum v surovom tvare `(2026-12-12)` — teraz je
`· 12.12.2026`. A **filter Event v Sales** dátum nemal vôbec; teraz ho má.
Dva koncerty tej istej šnúry sa inak v zozname nedajú rozoznať.

## 2.40.0 - tichšie stavy, čitateľnejší text, skeletony, návrat na riadok

Prvá dávka z tvojho výberu tridsiatich. Všetko je **frontend** — žiadna
migrácia, žiadny Rust, žiadna zmena v peniazoch.

### 17 · Badge je teraz bodka a text

Stav prestal byť plná farebná pilulka. Zostala **farebná bodka a obyčajný
popis** — bez výplne, bez rámika. V tabuľke, kde má každý riadok stav, tie
pilulky prekričali čísla vedľa seba.

Farby sa nemenili ani o odtieň: `STATUS_TONES` je stále jediný zdroj pravdy,
len sa z neho pri vykreslení vyhodí výplň a rámik (`quietTone`). Bodka je o
stupeň väčšia a v plnej sile, keďže identitu teraz nesie sama. **Ani jedno
volanie sa nemenilo.**

`InlineStatusSelect` si plnú pilulku **necháva zámerne** — je to ovládací
prvok a ten musí vyzerať ako ovládací prvok.

### 18 · Oprava kontrastu — 260 miest

Sivý text v tmavom režime bol na hrane čitateľnosti, a bolo to **prehodené
naopak**: tmavší odtieň na tmavšom podklade. Zmerané **3,16 : 1**; po oprave
**4,16 : 1**.

Zámena `text-slate-400 dark:text-slate-500` → `text-slate-500
dark:text-slate-400` prebehla na **260 miestach v 24 súboroch**. Overené, že
ani jeden výskyt nemal nalepenú predponu (`hover:`, `group-hover:`), takže
zámena nemohla nič iné zasiahnuť. Po nej: starý pár **0×**, správny **350×**.

### 16 · Skeletony namiesto „Loading…“

Zoznamy to vedia od 2.6.0. Dorobené tam, kde ešte zostávalo koliesko nad
prázdnom: **Dashboard** a **všetky štyri karty Finance**. Pribudli dva tvary —
`StatsSkeleton` (riadok kariet s číslami) a `PanelSkeleton` (karta textu alebo
grafu). Stránka si drží výšku a neposkočí, keď dáta dorazia.

V modáloch a krátkych čakaniach vnútri panelov `LoadingBlock` **zostáva** —
tam je koliesko správna odpoveď.

### 20 · Riadok, na ktorom si bol

Prilepená hlavička tabuľky **už v appke bola** (od 2.6.0, `.table-shell thead
th`). Druhá polovica tej karty nie — a tú robí táto verzia: keď sa vrátiš z
detailu, **riadok, z ktorého si odišiel, sa na chvíľu rozsvieti**.

`lib/lastRow.ts`, rovnaký dohovor ako `lastFilters` v Sales: modul-level,
platí do reštartu, databázy sa netýka. Značka sa pri čítaní **spotrebuje**,
takže riadok bliká raz po návrate, nie pri každej návšteve. Zapnuté na
**Events, Inventory a Sales** — v Pulls nie, tie vlastný detail nemajú.
Rešpektuje `prefers-reduced-motion`.

## 2.39.0 - Orders a Inventory sú jedna obrazovka; Margin zaniká

### Jedna obrazovka, volá sa Inventory

Orders a Inventory boli **dva zoznamy nad tými istými riadkami** — obe volali
`api.listOrders`, obe vykresľovali objednávky, obe viedli na `/orders/:id`.
Líšila sa len sada stĺpcov. Teraz je to jedna položka v menu, **Inventory**.

Prežil `/orders` (bohatšia stránka — New Order, úpravy, hromadné akcie) a
dostal názov Inventory. `/tickets` naň **presmeruje aj s query stringom**,
takže staré odkazy fungujú — vrátane `?code=` z detailu eventu, ktoré Orders
nájde, lebo jeho vyhľadávanie matchuje aj kódy lístkov.

Čo sa prepojilo: odkazy z Dashboardu (×2), z detailu eventu (×2), spätný odkaz
z detailu objednávky (už nevetví — vždy „Back to inventory") a prehliadka.
Prehliadka mala **dva kroky na `/tickets`**; jeden by teraz ukazoval tú istú
stránku dvakrát za sebou, takže sa zlúčil do kroku o objednávkach (veta o
listing price sa presunula doň). Zostáva 11 krokov a Guide si to číta sám.

`pages/Tickets.tsx` **nie je zmazaný** — SaleDetail a OrderDetail z neho
importujú `DELIVERY_STATUS_OPTIONS`, `RESALE_STATUS_OPTIONS` a
`TicketEditModal`. Odišla len jeho route a položka v menu.

### Margin zaniká

Marko: „margin vsade kde je uplne ju odstranme aj s widgetov proste ako keby
zanikla." Odišla odvšadiaľ, kde ju bolo vidieť:

- **Dashboard → Financials:** karta „Margin", aj podriadok pri Profite (ten
  teraz ukazuje len ROI).
- **Dashboard → porovnanie období:** riadok Margin.
- **Detail eventu:** karta Margin.
- **Detail predaja:** karta Margin (súhrn je päť kariet namiesto šiestich,
  mriežka sa zúžila s ním, nie je tam diera) aj výpočet.
- **Texty**, ktoré o nej hovorili — prehliadka (dva kroky) a Guide v Settings —
  hovoria o zisku, nie o marži.

**ROI zostáva.** Backend `margin` ďalej počíta a posiela, len to už nikto
nečíta — meniť `finance.rs` kvôli zobrazeniu by znamenalo siahať na chránený
finančný modul bez možnosti to tu preložiť.

`components/Recap.tsx` má maržu tiež, ale ten je od 2.38.0 nedostupný (nič ho
neimportuje), takže na obrazovku sa nedostane.

## 2.38.0 - riadok filtrov, vlastný kalendár, tenšie Settings

### Prepínač kariet ide do riadku filtrov

`Upcoming / Completed` (a `Pending / Completed`) už nesedí na vlastnom
riadku nad filtrami — je **vpravo v tom istom riadku, kde je Search a
Sort**, presne ako to má Pulls. Platí na **Events, Orders, Inventory aj
Sales**. `TabSwitcher` si prestal nosiť vlastný `mb-4`; tie dve miesta,
ktoré stoja samostatne (detail eventu), si ho píšu samé.

### Sales

- **Currency filter preč.** Dátumový rozsah **Od / Do** sa posunul presne
  na jeho miesto.
- **Refund status preč úplne** — a s ním aj tlačidlo „More filters", pod
  ktorým to bola jediná vec.
- **Jeden status na riadok.** Badge `Paid / Pending` zmizol, **guličky
  zostali** (Sold · Deliv. · Paid) — riadok hovoril to isté dvakrát. Riadok
  „N/M refunded" sa presunul pod guličky, nezmizol: je to jediné miesto,
  kde zoznam vôbec povie, že sa niečo vrátilo.
- Stĺpcov je desať namiesto jedenástich, obe `colgroup` prepočítané na 100.

### Inventory

Stĺpec **Sold preč**, zostáva **Total** a **Available**. Nič sa
neprepočítavalo — Total je stále počet kusov, Available stále
`available + listed`, takže predané je čitateľné ako rozdiel medzi nimi.
Obe `colgroup` prepočítané (8 stĺpcov naširoko, 6 nasúzko) a **Event pustil
kus zo svojich 43,5 %**, aby sa Seats a Purchase date zmestili celé.

### Vlastný kalendár, všade

`<input type="date">` doteraz otváral **kalendár prehliadača** — biely
panel, vlastné písmo, o tmavom režime appky nevie nič. Je to chrome
prehliadača, CSS sa k nemu nedostane. Takže je nakreslený vlastný:
mesiac + šípky, pondelok prvý, šesť riadkov (výška panela sa nemení),
dnešok zvýraznený, `Today` a `Clear` dole, otvára sa hore alebo dole podľa
toho, koľko je miesta. **Žiadna nová knižnica.**

`Input` posiela `type="date"` doň sám, takže **ani jedno z ~30 miest sa
nemenilo** — stále posielajú `value` ako `YYYY-MM-DD` a stále čítajú
`e.target.value`. Tri miesta, ktoré si kreslili `<input>` ručne
(Dashboard ×2, detail objednávky), idú teraz cez `Input`.

### Settings

- **Insights preč úplne.** Bola to jediná cesta k Recapu, takže z appky
  odchádza aj Recap; `components/Recap.tsx` zostáva na disku nedotknutý,
  len ho už nič neimportuje. Zmazať ten súbor je samostatné rozhodnutie.
- **Import CSV + Export CSV = jedna karta „CSV"** s dvoma riadkami.
  Handlery sú do písmena tie isté.
- **Guide prerobený.** Gradientový hero preč — bol to najhlasnejší prvok v
  Settings a hovoril najmenej. Zostalo to isté, čo niesol: tlačidlo na
  prehliadku a **tri veci, ktoré si nový používateľ pomýli**, teraz ako tri
  krátke bloky vedľa seba. Počet krokov sa berie z prehliadky samotnej
  (`TOUR_STEP_COUNT`), nie z čísla napísaného v texte.
- **Posledný krok prehliadky** mieril na `/settings/insights`. Neznáma
  sekcia ticho spadne na prvú, takže by prehliadka skončila na Lookups a
  rozprávala o recapoch, ktoré už neexistujú. Končí na Support, odkiaľ sa
  spúšťa.

### Čo to znamená pre refundy

Po 2.35.0 (tlačidlo Refund) a tejto verzii (filter Refund status) **už v UI
nie je ani jedna cesta k refundu — ani ho spraviť, ani si ich vyfiltrovať.**
Dáta aj logika sú nedotknuté, refundované kusy sa stále počítajú v riadku
predaja aj v súhrne dole. Píše sa to sem, aby to nebolo prekvapenie.

## 2.37.0 - dizajnový jazyk 3.0, prvá vrstva

Schválené na celoapkovom náhľade. Toto je **vrstva tokenov** — komponenty a
texty idú ďalej.

### Hĺbka tónom, nie rámikom

Tmavý koniec sivej rampy je prerezaný: **950 `#08080b`** je podklad appky,
**900 `#101016`** karta, ktorá nad ním sedí, a **800 `#1e1e27`** vlásočnica
namiesto rámika. Povrchy v `index.css` to nasledujú.

Karta dostala späť **veľmi jemný tieň** — jednu vrstvu, pätinu toho, čo tam
bolo pred 2.29.2. Vtedy som tiene odstránil, lebo appka vyzerala 3D. Opačný
problém — všetko rovnako ploché — rieši toto.

### Hlavičky tabuliek stratili verzálky

`.th`, `.th-c`, `.th-c-narrow` aj `.section-title` idú do normálneho textu,
váha zo `semibold` na `medium`.

**Rozmery som nechal.** `PROTECTED_AREAS.md` hovorí, že `px-1` a `text-[11px]`
boli merané proti reálnym dátam v troch jazykoch, aby nikdy nevyskočil
vodorovný posuvník. Zrušenie verzálok text **zužuje, nikdy nerozširuje**,
takže tá záruka platí ďalej.

### Kontrast — nameraný, ale neopravený

`slate-400` sa posunul zo **4,44 na 4,56** na bielom. Krok pre tlmený text tak
prvýkrát prechádza hranicou 4,5 vo svetlom režime.

**Tmavý režim sa v rampe opraviť nedá.** Žiadna hodnota nevyhovie na bielom aj
na `#101016` naraz — musela by byť zároveň tmavšia aj svetlejšia. Pár teda
musí byť rôzny podľa režimu. A tu je problém:

**Prevládajúci zápis v appke to má obrátene.** `text-slate-400
dark:text-slate-500` je na **258 miestach** a na tmavšie pozadie volí tmavší
krok — vyjde **3,16:1**. Správny tvar `text-slate-500 dark:text-slate-400` je
na 85 miestach a dáva 4,16:1.

Oprava je mechanická výmena 258 výskytov naprieč všetkými stránkami. To si
zaslúži **vlastnú verziu**, nie aby sa to prišilo k zmene tokenov.

### Čo z 3.0 ešte príde

`Badge` z vyplnenej pilulky na bodku + text (bodka tam už je a dedí farbu),
prekreslenie `Skeleton` a `EmptyState`, texty prázdnych stavov po stránkach,
a tá kontrastná výmena.

## 2.36.0 - Pulls patria k lístkom, akcent je fialový

Oboje si schválil na interaktívnom náhľade skôr, než som sa dotkol jediného
súboru.

### Navigácia

**Pulls sú v skupine Tickets**, piate za Sales. A nadpis **„Market & money"
zanikol** — po presune Pulls by stál nad jedinou položkou, čo nie je kategória,
ale ozdoba.

Finance je samostatná položka najvyššej úrovne a **pod Tickets zámerne
nepatrí**: pokrýva peniaze, ktoré s lístkami nemajú nič spoločné — nájom,
poplatky, osobné výdavky — a má vlastné štyri taby, kategórie aj účty.

**Žiadna routa sa nezmenila.** `/pulls` je stále `/pulls`, takže staré odkazy,
krok sprievodcu aj návraty z detailov fungujú ďalej. Zmenil sa jeden súbor:
`Layout.tsx`.

### Farba

Ružová z 2.34.0 je preč, rampa sa presunula na odtieň 253 — tá levanduľová
rodina z Onyxu, o stupeň hlbšie. Presne to, čo si vyklikal v náhľade.

**Dva kroky som nedopočítal, ale doladil — a oba kvôli kontrastu, nie vkusu:**

* **600** nesie po celej appke biely text (`bg-brand-600`) a má **9,1:1**.
  Predtým mala ružová 4,6:1, čiže toto je výrazne lepšie čitateľné.
* **400** je v tmavom režime farba odkazov a akcií (`dark:text-brand-400`).
  Keby som len posunul svetlosť, vyšlo by **4,05:1** na povrchu karty — pod
  hranicou 4,5. Nastavil som ho na **5,3:1**.

Sivá rampa ostala nedotknutá — už predtým mala fialový nádych a k novému
akcentu sadá.

### Čo som nerozhodol za teba

Tri veci z design review v náhľade, ktoré stále čakajú:

1. Skupina sa volá **Tickets** a je v nej položka **Inventory** — dve
   lístkové mená v jednom rail-i. Premenovať skupinu?
2. **Zabalenie skupiny teraz schová Pulls.** Predtým bol vždy viditeľný.
3. **Inventory stále vypisuje aj predané a refundované** lístky — filter
   odišiel s `/inventory` v 2.35.1.

## 2.35.1 - Inventory Intelligence preč, dve stránky lístkov sa zlúčili

### Inventory Intelligence

Celý blok zo stránky eventu je preč. **A s ním aj filter lístkov** — každé
kliknutie, ktoré ten filter zapínalo, bolo vnútri toho bloku, takže by sa už
nedal zapnúť a pás „Showing: ..." by sa nikdy neukázal. Tabuľka lístkov pod
tým zase vypisuje všetky.

Backend som nechal: `get_inventory_intelligence` aj `inventory_intelligence.rs`
existujú a sú zaregistrované. Zmizla len obrazovka.

### Tickets a Inventory sú jedna položka

Boli to vždy tá istá stránka — Inventory bol `TicketsView` so zamknutým
filtrom. Teraz je v sidebare jedna položka **Inventory**, a routa, ktorá
prežila, je `/tickets`. Preto ti fungujú staré odkazy aj návrat z detailu
objednávky.

Prerobil som **všetky** odkazy na `/inventory`, inak by viedli do prázdna:
krok sprievodcu, dlaždicu „Missing listing price" na Dashboarde (aj riadok, aj
kartu) a vetvu návratu v detaile objednávky vrátane popiskov.

### Jedna vec na rozhodnutie

Keďže `/inventory` zmizol, **nikto už neposiela `lockedStatus`** — a tým sú
tie dve vetvy z 2.35.0 nedostupné: filter „available + listed" a tých sedem
stĺpcov, ktoré si pre Inventory vypísal.

Nezmazal som ich. Ak má tá premenovaná stránka ukazovať len predajný sklad a
nie všetky lístky vrátane predaných, je to jeden prop a vrátim to. Povedz.

## 2.35.0 - stĺpce, gulôčky a opravené selecty

### Stĺpce podľa tvojich zoznamov

**Sales:** Sale, Event, Platform, Event date, Seats, Tix, Cost, Revenue,
Profit, Status, Completed. Fees a Margin/ROI sú preč, Cost je pred Revenue —
riadok sa číta náklad → čo to prinieslo → čo zostalo.

**Detail predaja:** Ticket, Order, Seat, Cost, Sale price, Profit a tri stavy.
Fees preč, Cost prvý.

**Inventory:** Event, Purchase date, Seats, Total, Available, Total cost,
Status. Order a Sold vypadli **len tam** — Inventory a Tickets zdieľajú jeden
komponent, takže som to podmienil cez `lockedStatus`, na ktorom ten súbor už
aj tak rozlišuje. Tickets ostalo nedotknuté. Dátum a sedadlá sa v Inventory
prestali skrývať pri úzkom okne, lebo po dvoch stĺpcoch menej je miesto.

### Tie polia na výber na Macu

Príčina nebola šírka stĺpca. `InlineStatusSelect` je `inline-flex` okolo
`<select>` s `appearance-none` — a taký select sa roztiahne na **najširšiu
položku**, nie na tú vybranú. Keďže každá bunka má `whitespace-nowrap`,
„Not delivered" vytlačilo celú pilulku cez susedný stĺpec. Preto sa ti tri
prekrývali.

Teraz má obal `min-w-0 max-w-full` a select `w-full min-w-0 truncate` —
zmestí sa do bunky a v najhoršom prípade sa oreže tromi bodkami. Stĺpce som
rozšíril tiež, ale to bol príznak; toto je príčina.

### Gulôčky sú teraz všade rovnaké

Vytiahol som ich do jedného zdieľaného komponentu `StatusDots`. Berie **ten
istý zoznam podmienok**, ktorý si každá stránka už aj tak stavia pre odznak —
takže bodky a odznak nemôžu tvrdiť nič rozdielne.

**Pulls** má dve (Paid, Transferred) a dajú sa klikať. **Sales** má tri (Sold,
Delivered, Paid) a klikať sa nedajú, lebo sú odvodené — kliknutie by bolo
klamstvo. Vzor je rovnaký, počet sa riadi dátami.

**Ako sa to číta bez hoverovania:** hlavička stĺpca ich menuje v poradí
(„Paid · Done", „Sold · Deliv. · Paid"), zelená znamená hotové, sivá nie.
Hover vypíše každú slovom.

### Refund

Odstránil som ho, ako si chcel. **Ale bol to jediný spôsob, ako sa v appke
dalo refundovať** — `OrderDetail.tsx` to má priamo v komentári. Všetko za ním
(dialóg, príkaz, stav „refunded", blok Refunded) ostáva nedotknuté, zmizlo len
tlačidlo. Vrátiť ho je ten jeden blok.

### Finance

Taby sú vedľa nadpisu, nie pod ním. O riadok nižšia hlavička.

## 2.34.1 - font naspäť

Mono si videl a nechcel. Stack je **presne ten, čo tam bol** — bajt po bajte
rovnaký ako vo všetkých verziách po 2.33.0 vrátane. Overil som to diffom proti
2.33.0 zipu, nie od oka.

Ružová (Vapor) a graf s plochou ostávajú, tých si sa netýkal.

Jedna vec z 2.34.0 platí ďalej a nechal som ju napísanú v kóde aj v
`CURRENT_STATE.md`, nech ju niekto o pol roka nehľadá znova: **`"Inter"` na
čele toho stacku sa nikde nenačítava.** Appka vždy kreslila systémovým písmom.
Nechal som ho tam, lebo jeho odstránenie by na obrazovke nezmenilo nič.

## 2.34.0 - Vapor

Vybral si si: Vapor, Spline Mono hrubší, graf Plocha, Finance Mix. Tri zo
štyroch sú vonku, štvrtá nebola treba.

### Celá appka je ružová

Levanduľa je preč. Prepísal som dve rampy v `tailwind.config.js` — `brand` na
ružovú, `slate` z modrosivej na fialovosivú pri rovnakých stupňoch svetlosti —
plus štyri hexy v `index.css`. Nič iné som nechytal: každý `bg-brand-600` a
`text-slate-400` naprieč appkou si nové hodnoty vezme sám.

`brand-600` som **nedal na najkrajšiu ružovú, ale na tú s kontrastom** — biely
text na nej má ~4,6:1. Sýty koniec rampy je na 400/500, kde ho tmavý režim
používa ako text na tmavom, nie ako text na akcente.

### Font: zlá správa a dobrá

**Zlá:** Spline Sans Mono do appky poslať nejde. TIQR neťahá žiadny webový font
a nemá zabundlovaný ani jeden — stiahnuť ho z Googlu by rozbilo offline chod.

**Dobrá:** pri hľadaní som zistil, že stack viedol `"Inter"`, ktorý **nikde
nie je**. Žiadny `@font-face`, žiadny súbor, žiadny odkaz. Appka celý čas ticho
padala na systémové písmo. Takže si doteraz Inter nikdy nevidel.

Teraz je tam **systémový mono** — SF Mono na Macu, Consolas na PC. Vyzerá
takmer ako Spline Mono a nepotrebuje sieť.

### Graf je plocha

`MetricChart` kreslí pod krivkou výplň s prechodom. Používa **tú istú cestu**
ako čiara, takže výplň nikdy nemôže ísť inokade než ťah nad ňou. Základňa je
nula — na P&L grafe je to tá čiara, na ktorej záleží.

### Finance som nechal tak, a je to zámer

Vybral si Mix. `finance/Overview.tsx` **ten tvar už má**: KPI pás, druhý KPI
pás, pod tým dvojstĺpec s rozpadom kategórií a grafom príjem/výdaj. To je
presne poradie z Mixu.

Rozdiel oproti ukážke sú proporcie, nie štruktúra. Prestavovať 591 riadkov
naslepo, bez možnosti zbuildovať, za kozmetický zisk sa mi nezdalo. Je to
odložené, nie prehliadnuté — povedz a spravím to ako ďalšiu verziu.

## 2.33.0 - menej obrazoviek, menej stĺpcov

### Events má osem stĺpcov

Event, Date, Status, Tickets, Stock, Cost, Revenue, Profit. **Days, Available,
Margin a ROI z tabuľky zmizli** — čísla nezanikli, sú ďalej na stránke eventu.
Preč je aj **farebný pruh kategórie**, ktorý pridala 2.32.0 na ľavú hranu
riadku; odznak kategórie vedľa názvu ostáva. Obe colgroupy sú teraz rovnaké —
keď zmizli Margin a ROI, širokému režimu nezostalo čo pridať.

### Price Checker, Ticket Center a Calendar sú preč z appky

Routy, položky v sidebare aj tri súbory stránok sú zmazané. S nimi aj všetko,
čo na ne ukazovalo:

* celá karta **„Market vs. mine"** a odkaz „Open in Price Checker" na stránke
  eventu,
* tlačidlá **„Check prices"** na detaile objednávky aj predaja,
* vetva návratu na **`/ticket-center`** z detailu objednávky,
* box **MARKET ATTENTION** na Dashboarde,
* dva kroky sprievodcu (zo 14 na 12) — a dva ďalšie som preformuloval, lebo
  sľubovali kalendár, ktorý už neexistuje.

### Z databázy som nezmazal nič

**Toto si prečítaj, je to jediné rozhodnutie, ktoré som spravil za teba.**
Povedal si „kompletne odstrániť" — odstránil som **obrazovky**, nie dáta.
Rust príkazy (`calendar.rs`, `price_checker.rs`, scanner aj analysis) ostávajú
a sú ďalej zaregistrované, `api.ts` ich ďalej ponúka, a tabuľky
`price_checks`, `price_check_tiers` a `event_marketplace_links` držia každý
riadok, ktorý držali.

Dôvod je jednoduchý: história skenov sa po zmazaní už nedá vrátiť. Vrátiť
obrazovku naopak stojí jednu routu a jednu položku v sidebare. Ak naozaj
chceš zmazať aj dáta, povedz to zvlášť a spravím migráciu.

## 2.32.0 - what the table shows you

Z tvojich 31 vybraných návrhov mala táto verzia priniesť trinásť čisto
zobrazovacích. Sedem z nich je vonku.

### Päť si už mal

Toto nie je výhovorka, je to výsledok čítania kódu pred písaním:

* **Miesto pod názvom eventu** — `Events.tsx` už vykresľuje `[venue, city]`
  ako druhý riadok pod menom.
* **Poznámka v Pulls** — „More info" je vlastný stĺpec už dávno.
* **Farba podľa druhu v kalendári** — `KIND_ACCENT` pokrýva všetkých sedem
  druhov.
* **Čipy filtrov v kalendári** — `activeKinds` a prepínacia lišta existujú.
* **Minulé/budúce v Events** — `EVENT_TABS` s `upcoming`/`completed`.

### Čo pribudlo

**Events** dostali stĺpec **Days** („3d", „today", „passed"), **pruh kategórie**
po ľavej hrane riadku a **Stock** — predané voči kúpeným ako pásik. Pruh berie
farbu z tej istej palety ako existujúci odznak kategórie, takže kategória nikdy
nemôže byť jednou farbou ako pilulka a druhou ako pruh.

**Pulls** majú Paid a Done v **jednom stĺpci ako dva body**. Obidva ostávajú
klikateľné — bodka má 10 px, ale tlačidlo okolo nej má normálnu veľkosť, lebo
toto odškrtávaš často. Tabuľka tým vrátila šírku textovým stĺpcom.

**Price Checker** označí sken starší než 7 dní jantárovo. Relatívny čas tam bol
aj predtým, ale čítal sa rovnako po hodine ako po mesiaci.

**Kalendár** má vedľa názvu obdobia súčet — „4 events · 7 deadlines". Počíta z
`visibleEntries`, takže keď vypneš druh vo filtri, číslo sa zmení tiež.

**Riadky tabuliek nabiehajú zhora nadol.** Jedno CSS pravidlo na `.table-shell`,
osem krokov a potom už nie — pri 200 riadkoch by si inak čakal na animáciu dlhšie
než na dáta. Systémové „obmedziť pohyb" ju vypína úplne.

### Nebezpečnú zónu som nepostavil

V appke nie je čo do nej dať. Neexistuje príkaz na zmazanie všetkého ani
továrenský reset. Jediné, čo prepisuje dáta, je Obnova a Sync down — a obidve
už majú vlastný `danger` potvrdzovací dialóg. Postaviť tú sekciu by znamenalo
buď prelepiť záchrannú akciu ako hrozbu, alebo dorobiť mazacie tlačidlo, ktoré
si nikdy nechcel. Ani jedno.

**Seats ostávajú ako boli** — pásik sedadiel (`t1`) si zrušil, vypadol z plánu
úplne.

## 2.31.0 - a pull is paid AND transferred

Marko: "do pulls taktiez mali by byt 2 veci dane co sa daju checknut a to je
payment ci uz zaplatili a + transfer ale ten tam uz je."

### The second checkbox

Pulls (Given) has a **Paid** column next to **Done**, tickable straight from
the list exactly like the transfer one. It answers the other half of a pull:
has the buyer actually paid the fee? The two are independent on purpose - they
finish in either order, and "transferred but not paid" is the state worth
seeing. Both are also correctable from the edit form.

`paid` is its own column with its own `paid_at` stamp (migration 030), shaped
exactly like `transfer_done`/`transfer_done_at`. One helper now applies the
timestamp rule for both: stamp when it really flips off->on, clear when it
flips back, and never touch it on a plain re-save.

### Completed now means both

**Every pull you already have will read "Not paid" until you tick it** -
including ones showing "Completed" today. That is the new meaning, not lost
data: nothing knew about payment before this version, so there was nothing to
carry over. The hover on the badge spells out which half is missing.

### The Google Sheet

Untouched. It has a `Transfer` column and no `Paid` one, so the sync neither
reads nor writes payment - it carries the app's own value through instead,
which is what stops a sync from quietly un-ticking a paid pull. Adding a real
`Paid` column would mean changing your live sheet, so that is your call.

**Not included, say if you want them:** a Paid filter next to the existing
Transfer filter, and payment tracking on Pulls (Received) - that tab has no
transfer flag either, so there was nothing to pair one with.

## 2.30.1 - Settings says less

Marko: "zjednodusti celu data sekciu a jednodusit vsetko co je v nastaveniach
nech je to minimalisticke."

### Twenty blocks of prose, cut

Nothing moved, nothing was removed, nothing changed what it does - only the
text around the controls. Cloud sync went from four sentences to one line. The
CSV importer no longer lists sixteen column names on screen; the template file
it tells you to download is where those actually get read. Export, Backup,
Drive revisions, restore points, the two Google sign-in notes, the Sheets URL
note, the Anthropic key, ntfy, Notifications, the guide header, Suggest a
change, both category pickers and all eight section descriptions are each a
line or two now.

What was deliberately KEPT, because each one changes what you would do:

* an import is all-or-nothing;
* notifications only fire while the app is running, one per category per day;
* an ntfy topic is a shared secret - anyone who knows it reads your alerts;
* duplicates are never flagged on import;
* a restore or a sync down replaces this computer's data, and takes a backup
  first;
* both-computers-changed: combining keeps everything, the other two throw one
  side away.

### The counter bug 2.30.0 left behind

Found while writing this up, fixed here, and it is the more important half of
this release. `reconcile_counter` exists so a merge can never leave a code
counter behind the codes the table actually holds - the failure it prevents is
silent for days and then stops order creation dead on a UNIQUE constraint.
2.30.0 started minting from per-event counters (`order:CELINE`) and did not
teach that function about them, so the exact failure it was written to prevent
came back one counter row over: `CELINE-004` arrives from the other machine,
this machine's `order:CELINE` still reads 3, and the next Celine order collides.

It now raises every event counter a table's codes imply, in one statement, and
there is a test named for that scenario. `counters` stays out of the merge on
purpose - each machine owns its own numbering.

**Known and left alone:** when two machines mint the *same* code for one event
at once, the arriving one is still re-numbered with the old fixed prefix
(`ORD-000092`), not the event's. It is rare, it is harmless, and making it
event-derived means a different event lookup per table - not something to do
inside a text-simplification release.

## 2.30.0 - CELINE-001: codes that say what they are

Marko: "ked je to celine dion tak celine-001 ... bolo by to lepsie zmapovatelne
nez to ako to je teraz", and "nech je vsade rovnaky - nie je jedno miesto ma 91
a druhy order 0091".

### One format, everywhere

`shortCode` is gone. 2.23.0 rendered ORD-000014 as `#14` in three list views
and nowhere else, which is exactly the "91 in one place, 0091 in another" he
reported. Every screen now shows the same string.

### Codes come from the event

`ORD-000084` becomes `CELINE-001`. Orders, tickets, sales, pulls and received
pulls all mint from the event they belong to - a sale through its ticket, a
pull off the event name it stores. Each kind counts from 1 within an event, so
an event's second order is CELINE-002.

The prefix rule is pure and deterministic, because **both machines mint
independently from synced counters** - if it ever answered differently on two
machines they would produce two codes for one record. It keys on the part
before the app's own " · " separator, so two Oasis nights share a run of
numbers; folds accents (Letná -> LETNA); and caps at eight characters. Run
against marko's real event names: OASIS, MILEY, CELINE, ENGLAND, COLDPLAY,
EAGLES, GARTH, ACDC, VBV08 and VBV10 - no collisions.

**An event whose name yields nothing usable keeps its old code.** No invented
prefix - `code` is NOT NULL UNIQUE everywhere, so "no prefix" can never mean
"no code".

### Existing records are rewritten too

Migration 029 adds `legacy_code`; the rewrite itself runs in Rust, tied to that
migration so it happens exactly once, on the same all-or-nothing path as every
other schema change. Numbering is by `id` within each prefix, so both machines
rewriting their own copy of the same data land on identical codes.

**Your Google Sheet keeps working.** It matches rows back by the code in its
"TIQR ID" column, and every one of those is now a stale ORD-000091 - without a
fallback the sync would reject the lot as "does not match any order in the
app". The matcher now tries the current code first and falls back to
`legacy_code`.

### The one thing this needed inside protected logic

A sale group took its identity from `MIN(s.code)`. That is the same row as
`MIN(s.id)` for every code this app has ever minted, because ids and codes
ascended together - but a batch spanning two events (which the
`COUNT(DISTINCT t.event_id) = 1` guard exists to handle) would have shown
COLDPLAY-003 as the group's code while `batch_id` said OASIS-001: identified by
one string, displayed under another. The group now takes its code from the
MIN(id) row. Verified against a real mixed-event batch - the old expression
returned COLDPLAY-003, the new one OASIS-001, matching batch_id. `batch_id`
itself is untouched.

### Verified by running it

The prefix rule and the whole backfill were executed against a real SQLite
database, not read: accents folded, per-event numbering correct, the unnamed
event left alone, `legacy_code` preserved, and the counters advanced so the
next Celine order is CELINE-003 rather than a collision.

**Before you update, sync both machines.** The rewrite is deterministic on
identical data; if the two have drifted they will each renumber their own copy
and the merge will reconcile by `uid` afterwards, which works but is noisier
than it needs to be.

## 2.29.5 - Ticket Center stays put, and Finance explains itself

### Ticket Center

The search row and the four category tiles now stay on screen and only the
table scrolls. `.table-shell` already scrolls inside itself under a sticky
header - it was just using the default 13.5rem inset, which assumes ordinary
page chrome. Ticket Center has a filter row AND four tiles above the table, so
the shell ran past the bottom of the window and the PAGE scrolled instead,
taking the tiles with it. Measured against the real chrome.

### Finance: the thing that looked like bad arithmetic

I audited it by running the real queries against a real database with edge
cases rather than reading them, and **the arithmetic is correct**: a
future-dated entry is correctly excluded from a balance, a transfer is
subtracted from one account and added to the other from a single row, inactive
and non-EUR accounts stay out of the EUR totals. Every figure came out exact.

What is NOT obvious is this: **an entry's account is optional**, and the three
screens then quietly use three different populations of the same rows.

- Overview's *Income / Expenses* count **every** EUR entry
- **Account balances** only move for entries that **have** an account
- **Reports** skips account-less entries outright

Each is right for the question it answers, and nothing on screen said so — so
Expenses and the balances underneath simply refused to reconcile, which reads
exactly like a miscount.

Fixed by **naming the difference, not by changing which rows count** — altering
that would silently move a number marko has been reading. Overview now prints,
in the same style the non-EUR note already uses: how many entries in the period
have no account, and the exact net difference they cause between that block and
the accounts above.

## 2.29.4 - Seven fixes from marko's own screenshots

- **Orders**: the *Tier / Level* field is gone from the New order form, and
  *Supplier* is gone from its preview. `tier` is still sent (null unless the
  AI import panel fills it) and `tickets.tier` still exists, so nothing
  downstream changed shape - only the field he never filled in.
- **Events**: *Artist / team* removed from the preview. It was showing a row
  for a field the form does not even offer.
- **Sidebar**: when a page inside the Tickets group is open, the **group now
  marks itself** too, so the rail shows both where you are and which group it
  belongs to. It was only opening, never highlighting.
- **Sidebar and segmented tracks are darker.** New `--surface-sunken` token:
  darker than the page, for chrome that sits UNDER the content rather than on
  top of it. Both were using the lighter muted surface.
- **Price Checker**: the whole selection mechanism is gone - the "Selected: N"
  bar, "Select all", and the per-row checkbox. It never led anywhere a single
  click did not: the scanner opens one visible window that marko drives
  himself, so "Check selected" could only ever open the FIRST one anyway.
- **Finance transactions now sort by when things actually happened.** Sorting
  on the date alone returned 0 for everything on the same day, and a
  comparator that returns 0 leaves rows in whatever order the two lists were
  concatenated - every entry, then every transfer. A morning transfer sat
  below an evening entry, and the order shifted whenever either list grew.
  Broken by `id` descending (the closest thing to a time this data carries -
  the date columns have no clock), with a final stable tiebreaker.

Removing the Price Checker selection also orphaned a `toggle` helper that
still called the deleted `setSelected` - caught and removed before shipping;
that one would have failed the build.

## 2.29.3 - BUILD FIX (a comma)

`tsc` stopped on `Events.tsx(24,1): Identifier expected`. My script added
`PreviewPanel` to an import list that already ended in a trailing comma, so the
file got `Textarea,` followed by `, PreviewPanel }` - a double comma. Both
import lists are now written out properly, one name per line, like the rest of
the file.

**The check that was missing now exists.** Brace and JSX balance both sail
straight past this: it is a token-level error, not a structural one, and the
file balances perfectly while failing to parse. There is now a pass that reads
every named-import list in `src` and requires each specifier to be a plain
identifier - it reports zero across the tree.

Version stays **2.29.3**: the failed build published no release, so the updater
has never seen this number. Same call as 2.16.0, 2.24.0 and 2.27.0.

## 2.29.3 - The layouts, not just the paint

Marko was right: 2.29.0-2.29.2 changed how the app is COLOURED and never
changed how the create forms and Settings are ARRANGED. The preview he approved
had split-preview forms and a sections rail; the app still had plain dialogs and
a tab strip.

### Split preview

`Modal` gained an optional `preview` slot. When a form passes one, the dialog
becomes two columns: the form on the left, **what it is about to create** on the
right, updating as he types. The preview column is sticky, so on a long form the
result stays on screen while he works down it. Under `lg` it stacks, where
side-by-side would squeeze both halves.

Optional on purpose - every one of the app's other modals renders exactly as it
did, and only the create forms opt in.

**Wired so far: New order and New event.** Both read the form's OWN state -
the order preview uses `summary`, the same memo the cost bar already used - so
the panel cannot disagree with what `submit()` actually sends. The order preview
also states the per-ticket cost, which is the thing that split is for.

**Not wired yet: New sale, New ticket, New pull, finance entries.** Each needs
its own few lines against its own state; the slot is there and the pattern is
set, but I am not claiming work I have not done.

### Settings: sections down the left

The tab strip across the top becomes a rail on the left, as in the preview.
Routes are untouched (`/settings/:section`), so every existing deep link - the
sidebar's "Account settings", the Dashboard's update pill - still lands exactly
where it did. Under `lg` it falls back to a horizontal scroller rather than
eating half a narrow window.

### Also

`border-line` and `bg-line-soft` exist now: with a hairline drawing every edge
in the flat design, that was worth a token instead of repeating
`border-slate-200 dark:border-slate-800`.

## 2.29.2 - Flat: the lighting comes off

Marko: "odstranme tu taku 3d svetlo co je za tym, nech to je ako keby bez
efektov jednoliate... tie farby take tmavsie aby mali lepsi kontrast."

### The extrusion is gone

Every surface was lit by a two-shadow pair. That pair is now **a 1px ring**.
One substitution in `--sh-card` / `--sh-raised` / `--sh-inset` flattens every
card, table shell, dialog and field in the app at once — because once the light
stops drawing an edge, the edge still has to exist, and a hairline does it
without adding depth.

**One exception, on purpose:** `--sh-overlay` keeps a real drop shadow. A
dialog genuinely floats over a dimmed page, and with no shadow at all the modal
and the backdrop merge into one dark mass.

The gestures that only made sense while things were extruded went with it: the
sidebar's current item is **tinted** rather than pressed in, the chosen tab is
**filled with the accent** rather than lifted, and a selected card gets a brand
**ring** rather than a dent.

### Darker, with the contrast put back

With the lighting gone, every bit of separation has to come from the fills and
the lines themselves, so the ramp was rebuilt for that job rather than for
softness: the dark ground drops to `#0d0e12`, the surface steps well clear of
it at `#15171d`, and `slate-800` is **lifted** so a hairline is actually visible
against a panel — that hairline is now what draws every edge in the app.

Light mode goes back to a white surface on a light grey ground, which is the
highest-contrast pairing there is and no longer contradicts anything: the "one
sheet of material" rule that made white wrong in 2.29.1 was a property of the
soft design, and the soft design is what just came off.

### Not changed

No logic, no schema, no migration (next is still 029), no new dependency. Every
figure on every screen is the same figure.

## 2.29.1 - Onyx, second pass: the surfaces the first pass missed

Marko, on 2.29.0: the colours are not nice in the layout, New order / pull and
the settings are not changed, and it looks different in the real app than in
the preview. All three had the same root cause, and my 2.29.0 note that this
was "a two-file change" was wrong.

### `bg-white` was painted in 39 places

`ui.tsx` does not use `.card` — **Modal, ConfirmDialog, the shared panel, the
secondary button, the tab strip and the empty state each define their own
surface inline**, as `border border-slate-200 bg-white`. So every dialog in the
app — New order, New pull, New sale, every Settings dialog — kept its old flat
white box while everything around it went soft. Twenty-five more literals sat
across fifteen page files.

Fixed properly rather than one class at a time: a semantic **`surface`** colour
now resolves to the same `--surface` variable index.css already sets per theme,
and every `bg-white` became `bg-surface`. `text-white` and
`ring-offset-white` were deliberately left alone — those are genuinely white.

That is also the whole "looks different in the app" answer: on the new grey
ground a pure white card is the exact opposite of one sheet of material.

### The ground and the surface are no longer the same colour

2.29.0 made them identical, which is soft-UI orthodoxy and reads **washed out**
in light mode. The card is now a step LIGHTER than the page — which is what a
surface tilted toward a top-left light actually does — and every shadow pair
was re-measured against the card's own colour rather than the page's.

### More pigment in the accent

The first lavender was so desaturated that a primary button read as another
grey panel. The accent is the one saturated thing on screen and has to earn it.

### Also

Modal header and footer bands, tab strips and the empty state now use the
material's own gestures — a band is a muted surface, a chosen tab presses in —
instead of borders and tints. The page header's hairline rule went: in a design
with no borders it was the last stray line.

Checked in a browser against the real token values, in both themes, before
shipping. No logic, no schema, no migration (next is still 029).

## 2.29.0 - Onyx: the app becomes one soft material

Marko picked it out of a design lab of eleven, then out of ten siblings of the
one he liked: **Onyx** — a dark, near-neutral soft UI with a single lavender.

### It is four files, not twenty-three thousand lines

The 2.6.0 redesign put the whole visual language in four places on purpose, and
this release is the proof that it worked. Every `bg-slate-N`, `text-slate-N`,
`shadow-card` and `rounded-xl` already spelled out across the pages picks the
new look up **with no page edit**:

- **`tailwind.config.js`** — the `slate` ramp is retuned to near-neutral greys
  and deliberately FLATTENED at the dark end (900 and 950 now sit two values
  apart, not eight), because in this material the card and the ground are one
  sheet. `brand` becomes the lavender. `borderRadius` grows.
- **`src/index.css`** — the surface tokens, and the component classes.

### The shadow is the whole design

A soft surface is lit by **two** shadows derived from the ground it sits on:
one darker, one lighter. That pair cannot be a single fixed value — light and
dark need their own measured pairs — so `shadow-card`/`shadow-raised`/
`shadow-overlay` now resolve to `--sh-*` variables set per theme. Both themes
got measured pairs; the light half on a dark ground has to be a genuinely
lighter grey rather than a translucent white, which is the thing most dark
neumorphism gets wrong and why it usually looks muddy.

That also retires the 2.6.0 top-highlight trick: on a dark ground the light
half of the pair IS the highlight.

### What lost its border

`.card` and `.table-shell`. A soft surface that also carries a 1px outline
reads as two design languages arguing — the shadow pair already says where the
card ends. `.input` went further: a field is now a dent pressed into the sheet
rather than a box on it. The focus halo is unchanged and simply stacks on the
inset, so a focused field still reads instantly.

The sidebar's current item is **pressed in** instead of tinted — the one
gesture this material has that a flat one does not. The accent bar 2.6.0 added
stays, because the dent alone is quiet in a narrow rail.

### One reversal, stated plainly

`borderRadius` grew: `lg` 8px → 13px, `xl` 10px → 20px. That **reverses
marko's own 2.6.0 instruction** about "obrovské rounded cards". He chose the
preview that has them, and the reason is structural rather than fashion: a
soft extruded surface at a 10px radius reads as a mistake, because the corner
has to be round enough for the two shadows to travel around it.

### Not changed

No page file was touched. No logic, no schema, no migration (next is still
029), no new dependency, and not one number on any screen moves.

## 2.28.0 - Market Map removed

Marko: "odstranme tu mapu kompletne lebo aj tak to vbc nejde." Gone, not
hidden - both files deleted, every reference with them. `grep` for MarketMap,
market_map, marketMap or price_checker_map across `src` and `src-tauri/src`
returns nothing.

**Removed**
- `src-tauri/src/commands/price_checker_map.rs` (module, `compute_market_map`,
  13 tests) and its registration in `commands/mod.rs` + `lib.rs`. Commands are
  back to **181**.
- `src/components/MarketMapView.tsx`.
- The `MarketMap*` structs in `models.rs` and the matching interfaces in
  `types.ts`.
- `computeMarketMap` in `api.ts`.
- The event-level map state, the `scanTotal` memo, its effect and its render in
  `PriceChecker.tsx`.

**Deliberately KEPT** - none of this was the map, and all of it still works:
- **The automatic scan run** (`start_price_scan_run`): one press reads the
  whole page, on a background thread, while TIQR stays usable. Its notification
  and toast now say **"Price scan finished"** instead of advertising a map that
  no longer exists.
- **The viagogo reader label fix** in `price_checker_scan.js` - a real scanner
  bug found during the map's reader audit. Viagogo pages were labelling every
  listing `"generic"`; that stays fixed.
- **The truncated macOS user agent fix** ("An outdated browser...").
- **Scans saving themselves to history**, and the stripped-back marketplace
  card.
- **The `grid-cols-5` collision fix** (`EUR1,815.0064`).

**Version bumped FORWARD to 2.28.0**, not reused - marko's own rule for a
revert, because the updater rejects a repeated number.

## 2.27.0 - BUILD FIX (a comment I wrote, twice over)

The 2.27.0 build failed in `tsc` before anything else ran. Six errors, all in
`MarketMapView.tsx`, all cascade from one:

**`{/* ... */}` is only legal as a CHILD of a JSX element.** I put one directly
in a ternary's branch - `) : ( {/* ... */} <div>` - which is expression
position, where it is a syntax error. Fixed.

And the first fix was wrong in a second way worth recording: I rewrote it as a
block comment whose TEXT quoted the JSX comment syntax, so the comment
terminator inside my own explanation closed the comment early and broke the
file again. It is line comments now, which cannot be closed by their own
contents.

Both are the same shape as 2.24.0's `"Mixed events"` inside a non-raw Rust
string: **a comment whose content is also syntax**. Scanned the whole `src`
tree for the JSX case afterwards - zero remaining.

Version deliberately stays **2.27.0**: the failed build published no release,
so the updater has never seen this number. Same call as 2.16.0 and 2.24.0.

## 2.27.0 - One press reads the whole page, and the glitching is gone

### The glitching had two causes, both removed

1. **The map was unmounted on every refresh.** The map is recomputed after each
   pass of a scan, and the panel swapped itself for a loading box each time:
   the map vanished, a short box took its place, everything below jumped up,
   then the map came back and everything jumped down - several times a minute.
   Now a refresh is invisible except for the word "updating" in the header; the
   big loading state is only for the FIRST build, when there is genuinely
   nothing to show.
2. **The `zoom` control.** `zoom` is a non-standard CSS property that re-lays
   out the whole subtree, and it was re-applied on every render. It is gone -
   marko asked for the map to be "nehybne". Blocks are a fixed readable size,
   the area scrolls, and the panel has a fixed minimum height so adding a
   section no longer resizes it. Hover and colour transitions on the blocks
   went too.

### One press now reads the whole page, in the background

`start_price_scan_run`: scan, scroll, scan, until the page stops giving
anything new. It returns immediately and the run continues on a backend thread,
so **leaving the Price Checker page - or putting TIQR behind another window -
does not interrupt it**. Each pass broadcasts the same scan-result event a
manual scan always did.

**This is not the Live Market Monitor coming back.** There is no schedule, no
timer, no polling, and nothing runs unless marko pressed a button on a window
he opened himself. A run is finite by construction and ends on: an exhausted
page (three passes with nothing new - three, not one, because a lazy-loading
page routinely needs a beat), a 60-pass cap, a 300-second cap, a failure, or
Stop. When it ends it is over; nothing re-arms it.

The termination logic was executed against scripted page behaviours - a normal
page, an infinite feed, a dead page, a mid-run failure, a closed window, Stop,
and a page with a two-pass lazy gap. Every one terminates, for the right
reason.

### It tells you when it is done

An OS notification ("Market map is ready - N listings read") reaches marko with
TIQR behind another window, and an in-app toast fires wherever he is in the app
- the listener lives in the Layout, not on the Price Checker page he has
already left. A run he stopped himself announces nothing.

The card also reports **why** the run ended in the backend's own words, rather
than just "done" - a run that hit a cap says so.

### The "outdated browser" warning

macOS only, and it was a TRUNCATED user agent, not an old engine. WKWebView's
default string stops after `AppleWebKit/605.1.15 (KHTML, like Gecko)` with no
`Version/… Safari/…` suffix, so a site's browser check finds no version and
falls through to "outdated". The scanner window now sends the suffix the engine
leaves off - it says Safari/WebKit, which is exactly what renders the page.
Windows is untouched: WebView2 already reports a current version.

It defeats nothing - challenge pages are still detected and reported honestly,
never bypassed.

### Verified

`WebviewWindowBuilder::user_agent` and `WebviewWindow::eval` are both new to
this codebase, so both were checked against the real tauri 2.11.5
documentation rather than assumed.

## 2.26.1 - Price Checker stripped back to what marko actually reads

His screenshots, his words: the screen was unreadable. Three marketplace cards
side by side, each stacking a listings table, Min/Max filters, a CSV export, a
"Save to history" button, a Market Map, a Market Analysis panel and a "Compare
a specific ticket" tool underneath it.

### The manual "Check Prices" form is gone

Button and modal both. It opened a form asking him to type in the lowest,
median, average and highest price - **numbers the scanner had just read for
him**. So scanning is now the only way a price check is recorded, and **a
finished scan records itself**.

The auto-save waits for the tier breakdown so history keeps its "1 tier" line,
and it is guarded on the scan NUMBER rather than a boolean, so one scan can
never be written to history twice. **No currency, no save** - a price check
whose currency had to be guessed is worse than none, and the card says so in
one line instead of saving a guess.

### A marketplace card is now a scan button and its history

Exactly the last screenshot he sent: name, URL, Visible Scanner, Latest check,
the five figures, the history table. Everything else went - 766 lines of UI
that is no longer reachable was deleted, not just hidden.

### The Market Map moved out and became ONE map

Marko: "mapa by mala byt niekde inde nie tam dole a mapa by mala byt pre vsetky
platformy rovnaka a tie listingy sa spoja."

So `compute_market_map` is keyed on the **event** now, not on one scanner
session: every open session for that event contributes its listings to one map,
and it is drawn **above** the marketplace cards rather than inside one. Scan
Viagogo, then Ticombo, and section 102 shows both marketplaces' rows together.

Deliberately NOT deduplicated across marketplaces - the same seat listed on two
sites is two real offers at two real prices, and the scanner has no cross-site
listing identity that could tell a genuine duplicate from two different sellers
who happen to match.

### The collision in the screenshot was a real bug

`EUR1,815.0064` - the Highest figure running straight into the Listings count.
The stats row was `grid-cols-5`: five FIXED columns no matter how wide the card
is. These cards sit three-across, so a fifth of a third of the page is not
enough for a four-figure price. Now the column COUNT follows the width
(`auto-fit` + a `6.5rem` minimum), so the figures wrap to a second row and
every one of them is fully readable. Same class of bug as `.summary-bar` in
2.19.0, same fix.

## 2.26.0 - Price Checker Market Map

A second VIEW of a scan session that already happened. **No new scanner, no new
table, no migration, no background work, no polling** - `compute_market_map` is
the same shape as `compute_market_analysis` next door: it folds the listings a
manual scan already accumulated together with marko's own unsold tickets for
that event, and returns a read-only structure the frontend draws.

### The reader audit came first, and it changed the design

`price_checker_scan.js` is ONE generic DOM reader, not three per-marketplace
ones - the three `read*` functions differ only in the label they stamp. Per
listing it can produce price, currency, section, row, tier, quantity, listing
id and marketplace; everything except price and marketplace is best-effort and
routinely absent. It produces **no seat numbers, no venue identity, no
coordinates or map geometry, and no per-listing URL** - none of those exist
anywhere in the extraction.

So the map stops at tier -> section, section detail shows Seats as a COUNT, and
the only URL offered is the event's own marketplace link. A geometric replica
would have had to be invented.

### Bug found by the audit and fixed

**`hostFamily` had no viagogo branch.** Viagogo replaced StubHub as the seeded
marketplace in migration 017 and StubHub was removed outright in 020, but this
file was never updated - so every scan of a viagogo page fell through to
`"generic"` and every listing it produced was labelled `"generic"`. Since the
three readers differ only in their label, the fix is the label (three lines):
a `readViagogo`, a `hostFamily` branch and a dispatch arm. Nothing else in the
scanner was touched.

### Normalization: safe only

`"Sec 102"`, `"Section 102"` and `"102"` become one block. Trim, collapse
whitespace, upper-case the grouping KEY, and drop leading zeroes ONLY when what
is left is entirely digits - so `"0102"` joins `"102"` while `"0A"` stays
`"0A"`. Every group keeps both a `key` and the first SOURCE value as its
`label`, which is what is drawn. **Tier wording is never mapped**: "Level 100"
and "Tier 1" stay separate groups, because nothing here can know whether they
mean the same thing.

### What the map shows

Tier bands of section blocks, tinted in four steps by listing count (not a
continuous ramp - it is "more here than there", not a measurement). Marko's own
tickets overlay the same blocks in emerald and are counted separately, never
folded into the market count. Click a block for the detail panel: counts,
lowest / median / highest, then Row | Seats | Marketplace | Price | Mine, with
his own tickets first and each linking to the EXISTING order detail.

Filters: each marketplace, My tickets, and tier. Zoom 70-160%, scroll contained
inside the map area.

### Honest states rather than invented ones

- A scan with prices but **no sections at all** says so and draws nothing.
- Listings with no section go to one explicitly labelled bucket **outside** the
  grid, never scattered into blocks.
- A section whose listings span **more than one currency** reports no
  lowest/median/highest at all rather than a blend.
- Section order is numeric-then-alphabetical - a stable way to find a section,
  never a claim about where it physically sits.

### No pricing, anywhere

Lowest/median/highest describe what is listed in one block. Nothing compares
one section to another, interpolates between them, or recommends a price.

## 2.25.0 - A recap you watch, a guide that points, and the CSV gaps

Seven things marko asked for in one pass.

### The Recap is a story now

It OPENS as a sequence: a title beat, then one idea per screen, each arriving
with its own effect and advancing on its own, ending on a card that hands you
the full report. "Skip to the full report" reaches the old view at any point,
and "Replay" in the header starts it again.

Not one new figure. Every slide reads a field the report already shows, from
the same single `get_dashboard` call, so a slide and the report cannot
disagree. A slide with nothing true to say is never built - an empty period
says so instead of showing zeroes, and the two numbers the app genuinely does
not hold (what unpaid orders total, per-event profit inside a period) say so
on the slide.

### All time, and it is the default

Marko: "ked som chcel aby mi vsetko ukazalo tak som musel dat last 6 months. a
realne pracujem na tiqr mesiac max dva." With one or two months of history the
recap opened on a near-empty month and the only preset that showed the whole
business was a six-month window. `period_bounds` already had an `all` arm, so
this is the same two sentinel dates it produces and no backend change at all -
`previous_period_bounds` already special-cases them, which is why the summary
table correctly says there is nothing to compare against.

### The guide walks the app instead of describing it

The seven-paragraph list is gone. In its place is a 14-step tour that
NAVIGATES to each real page, finds a real element, dims everything else and
puts the explanation next to the thing it is talking about - the Dashboard's
headline numbers, the period picker, then events, orders, tickets, sales,
inventory, pulls, finance, price checker, calendar, sync and the recap.

Elements opt in with a `data-tour` attribute. Four files carry them and every
one is a single attribute with no logic: `PageHeader` in ui.tsx (which gives
every page in the app an anchor in one edit), the sidebar in Layout.tsx, and
two on the Dashboard. A step whose anchor is not on screen still runs, just
centred and without a spotlight - the tour never points at nothing and never
waits for an element that is not coming.

### Insights and Support moved to the very end of Settings

They are the two things you go looking for on purpose; everything above them
is something you came to Settings to change.

### The restore lists close behind you

The Drive revision list had no collapse at all - it loaded every version
Google still holds and left them all on screen. It now shows two with "Show N
more", both lists have a real **Hide**, and leaving the Data section puts them
back the way they were found. Switching Settings sections does not unmount the
page, which is why this is keyed on the section rather than left to unmount.

### CSV: three columns that existed in the database and reached no export

- **Orders** gained `event_date` and `external_reference`. The reference has
  been on orders since migration 009; the event's own date is the one 2.23.0
  made the visible date in the Orders list, while the export still carried
  only `purchase_date`.
- **Tickets / Inventory** gained `event_date`.
- **Sales** gained `event_date`.

All **appended at the end**, so every column an existing sheet or script
already reads keeps its position. A TBD event still exports a blank date -
never a fabricated one.

**Checked and deliberately NOT changed:** Events CSV is complete (its
`category` column is kept in step with `category_id` by `resolve_category_name`,
so it is not the stale legacy value it looks like). **Not built, and marko
should say if he wants them:** there is no Pulls export, no Finance export and
no ticket-listings export at all, and Sales still does not carry `batch_id` -
that one is protected and was left alone rather than touched in passing.

### Suggestions can carry a picture

One optional image, resized on a canvas before it is sent - no image library,
no Firebase Storage, no new dependency. It is stored in the suggestion
document itself, which is why the ceiling is 600k base64 characters: Firestore
allows 1 MiB per DOCUMENT and the text and metadata share it. The quality
steps down until it fits rather than refusing a big photo. The admin inbox
renders it, and only a `data:image/` URL is ever put into an image element.

**`firestore.rules` must be pasted into the Console again** - the new rule caps
the image field there too, because a rule that trusts the app to have resized
the picture is not a rule.

## 2.24.0 - BUILD FIX (three compile errors, no behaviour change)

The 2.24.0 build failed on both Windows and macOS. Three independent errors;
the other 34 lines of the log were cascade from the first.

1. **`commands/sales.rs` - a `"` inside a `&str` constant.** The 2.23.0
   `event_date` change added an SQL `--` comment containing `"Mixed events"`,
   `"no single date"` and `"-"` INSIDE `GROUP_BASE_SELECT`, which is a plain
   (non-raw) Rust string literal. The first quote closed the string and the
   rest of the SQL was parsed as Rust, which is why all 17 `sales::*` commands
   then "could not be found" by `generate_handler!`. Quotes removed from the
   comment text; the SQL itself is byte-for-byte unchanged.

2. **`commands/cloud_sync.rs` - `AtomicBool` was never imported.** Line 77 read
   `use std::sync::atomic::Ordering;` while `SYNC_IN_PROGRESS` (2.18.0) needs
   both. Widened to `{AtomicBool, Ordering}`.

3. **`commands/cloud_merge.rs` - `db_path` was used but never bound.**
   `cloud_merge_pull` called `merge_inner(&mut conn, db_path)` without the
   `let db_path = state.db_path.lock().unwrap().clone();` line its twin
   `cloud_sync_pull` has. Added, in the same lock order (db first, then
   db_path) so the two entry points cannot deadlock against each other.

**Errors 2 and 3 date from 2.18.0, not from Recap** - which means every build
from 2.18.0 onward failed the same way and 2.17.0 was the last version that
actually produced installers.

Version deliberately stays **2.24.0**: the failed build published no release,
so the updater has never seen this number and reusing it is not a repeat.
Same call as the 2.16.0 build fix.

## 2.24.0 - TIQR Recap: Ticket & Finance

**No schema change, no new dependency, and - the point of the whole thing -
NOT ONE NEW BUSINESS CALCULATION.** Reached from **Settings → Insights**, never
the sidebar.

### One existing call does all of it

Every figure comes from `get_dashboard` with a concrete range. That command
already returns the period's `FinanceSummary`, the equal-length previous
period, the cashflow split, the inventory potential, the per-platform
breakdown and the time series - so the Recap makes **one round trip**, not the
"dozens of separate queries" the brief warned against, and every number is the
same number the Dashboard shows under the same definition.

**Caught before shipping:** `period_bounds` in `dashboard.rs` matches on the
period NAME first and only reads `from`/`to` under `"custom"`. Passing the
dates alone falls into the `None` arm, which returns **today → today** - every
recap would have shown a single day's figures under a month's heading, silently
and plausibly. Checked against that function rather than assumed.

### Realized / Pending / Potential, kept apart by construction

Three bands, each with its own heading, colour and **one-line definition on
screen**, because without it "pending" and "potential" read as the same kind of
number:

- **Realized** - money that actually moved, inside the period.
- **Pending** - owed either way and not moved yet. *Unpaid orders show a COUNT
  only, and say so: the app tracks which orders are unpaid but never totals
  what is owed, and adding that sum would be a new calculation.*
- **Potential** - what unsold stock might be worth. Never called profit.

### The summary table

Compares against the previous period with the app's own `computeTrend` /
`computeTrendPoints`, so **percent and percentage points are distinguished**:
money and counts move by %, ROI and margin are already percentages and move by
**pp**. "Previous period" is the app's own definition - the equal-length window
immediately before - and the table says so underneath rather than leaving it to
be assumed.

### Charts and share

The chart is `MetricChart`, the app's own hand-rolled SVG component, unchanged.
Purchased/Sold/Remaining is a composition bar and deliberately **not** a time
series: nothing in the existing aggregation reports how many tickets were
*bought* per bucket, so that line would have to be invented.

**Share** produces a PNG, laid out by hand as SVG and rasterised on a canvas -
so it reads as a report rather than a screenshot of the app, and needs no
screenshot library (this project carries no UI dependencies). Saved through the
normal save dialog; `save_png_file` decodes base64, **verifies the PNG
signature** and writes bytes. No logic, no database, no cloud.

### Deliberately absent rather than invented

Biggest single sale, fastest-selling event and best tier: none exists as an
aggregation today, and building one would make this a reporting engine. The
best-event line is labelled **all time**, because all-time is the scope the app
already keeps per event.

### Verified

- **Date ranges executed, not reasoned about**: end of March → Feb 1–28; leap
  year → Feb 1–29; January → December of the previous year; rolling 3/6-month
  windows; custom keeps what was typed.
- 3 tests on `save_png_file` (signature accepted, non-PNG refused without
  writing a file, bad base64 is a clear error not a panic).
- 180 commands still match `api.ts` ↔ `lib.rs` in both directions.

## 2.23.0 - Event dates in lists, readable codes, the wheel works, and a Support section

**No schema change, no new dependency.** Five things marko asked for.

1. **Lists show the EVENT date, not the purchase/sale date** (Orders, Sales,
   Pulls). The list is scanned to find *what is coming up*; a purchase date
   answers a question nobody asks while scanning. Detail screens still show
   when it was bought or sold, and in the list it moved into the tooltip so
   nothing was lost. Sales needed `event_date` added to its grouped query -
   carried under the same "only when every line's event agrees" guard the event
   name already used, so a mixed-event group and a TBD event both honestly show
   "-" instead of a made-up date.
2. **Codes are readable in lists**: `#14`, not `ORD-000014`. Six zero-padded
   digits behind a prefix were the first thing the eye landed on in every
   table, and the table is already called Orders. **Only the display changed** -
   the stored code keeps its full form because a connected Google Sheet shows
   it and the merge parses its numeric tail to keep both machines' counters in
   step (PROTECTED_AREAS 2.16.0). Full code stays in the tooltip and on every
   detail screen.
3. **Finance → Transactions can be scrolled with the wheel again.**
   `.table-flush` is `overflow: auto` **plus** `overscroll-behavior: contain`,
   and this wrapper had no height - so it could never scroll itself while
   `contain` still stopped the wheel reaching the page behind it. Only dragging
   the window's scrollbar worked, exactly as reported. **Measured both ways in
   a real browser before fixing:** without a height cap the wheel moved the
   page 0px; with one the table moved 500px. The class's own doc comment
   already says it belongs on a box that scrolls - this usage never gave it
   one. **The same bug was in the Reports and Accounts tabs** and is fixed
   there too.
4. **Restore points show the last two**, with "Show N more". Each one that can
   be paired with a merge from the same minute now says what actually arrived
   ("then: 3 orders, 4 tickets") - taken from the merge log, and left blank
   rather than guessed when nothing matches (a whole-file sync down has no
   merge entry at all).
5. **Settings → Support**: a short guide in the order the app wants to be used,
   and a suggestion box. The box posts one document to Firestore and reads
   nothing back - a postbox, not a forum, because a reply thread that never
   gets answered is worse than none. Only an admin can read the collection, and
   `admin` is a field only the Firebase Console can set, so the app can never
   promote itself. The inbox appears in Settings for that account and nowhere
   else.

**Two manual steps for #5, and nothing in the app can do either:** publish the
updated `firestore.rules` in the Firebase Console, and set `admin: true` on
your own `users/{uid}` document. Until both are done, sending a suggestion
fails and the inbox stays hidden - the safe way round. The failure message says
so in plain words rather than showing a raw Firebase permission code.

## 2.22.0 - 2.21.0 reverted at marko's request

**The dashboard is back to exactly what it was in 2.20.0.** marko looked at the
preview and said so plainly: *"okej toto ani nejdem stahovat je to strasne
zabudnime na tuto verziu"*. He never installed it, so nothing needs migrating
back - the code is simply gone.

Removed: `OperationsSnapshot` and its five structs, the six aggregates in
`dashboard.rs`, the six Overview cards, and profit on the Sales-by-platform
card. `PROTECTED_AREAS.md`'s 2.21.0 section went with them - it documented
invariants for code that no longer exists, and a stale invariant is worse than
none.

**Untouched:** everything from 2.14.0 through 2.20.0. Automatic sync, the
merge, tombstones, merge history, AI cost, Drive revisions, the `.summary-bar`
grid fix, the busy indicator - all still there. So is `PeriodComparisonCard`
(DSH-L), which marko picked himself.

Version goes FORWARD, not back to 2.20.0: the Tauri updater compares version
numbers directly and will not accept a repeat. Same lesson as the 2.3.0 ->
2.3.1 revert (see `CURRENT_STATE.md`).

*What was actually wrong with 2.21.0 is not recorded here, because I do not
know yet - "it's terrible" could be too many blocks, the wrong metrics, or the
wrong idea entirely. Whatever replaces it starts from an answer, not a guess.*

## 2.21.0 - The dashboard starts answering questions

**No schema change, no new dependency, no API wired up - that last one was
marko's only constraint, and it turned out not to cost anything: nearly
everything a reseller needs was already in the database and simply never asked
for.**

### Six new aggregates in `dashboard.rs`, all from data already stored

The metric set is the one this trade actually uses, checked against published
inventory/ticketing KPI guidance rather than invented:

1. **Sell-through** (sold / bought) - that guidance calls it the single most
   important number in a stock business. `None` when nothing has been bought:
   a rate with no denominator is unanswerable, not 0%.
2. **Days to sell**, median *and* mean, with the sample size shown. The median
   because one ticket that sat for a year drags a mean into uselessness; the
   sample size because an average of three is not the same claim as an average
   of three hundred. A sale dated before its own purchase is left out - that is
   a typo, not a fast sale.
3. **Ageing stock** in 0-6 / 7-29 / 30-89 / 90+ day buckets, with the money in
   each, plus the five oldest tickets by name.
4. **Money at risk**: events in the next 14 days with unsold stock, ranked by
   euros and days left, and how many of those tickets have **no price at all**.
   The Attention tiles count things; this says which one costs the most to
   ignore. Same window and same `upcoming` scope as those tiles, so the two can
   never disagree.
5. **Realized profit per event**, sold tickets only - never blended with the
   potential profit of unsold stock.
6. **Where stock comes from**, per supplier. Only the SOLD tickets' cost counts
   against realized revenue, or every batch still in stock would show as a
   loss; and tickets bought with no supplier keep their own row rather than
   being folded into a neighbour, which would make every puller look worse than
   they are.

### On the screen

Six new cards on Overview, ordered by the question they answer: what happened
to the money, what the stock is doing, what needs doing today, and only then
what worked.

- **Where the money went** - revenue split into stock cost / platform fees /
  profit. It says out loud when no fees are recorded, because then the margin
  shown is a best case rather than a fact (marko's recorded fees are currently
  zero against €5,093 of revenue).
- **Sales by platform now shows profit beside revenue.** `profitCents` has been
  sent with every dashboard load since 2.0.47 and was never displayed, so the
  card ranked channels by turnover instead of by what they earn.

### How the numbers and colours were checked

- **All six SQL statements were extracted verbatim from the Rust source and run
  against a real seeded SQLite database** before being trusted - 6/6, with the
  figures hand-checked.
- The three bar colours (brand / amber / emerald, the app's own ramp - no new
  palette) were checked for colour-blind separation by **computing OKLab
  distance under simulated deuteranopia and protanopia**, not by eye: worst pair
  9.3 against a target of 8, normal-vision floor 23.8 against a floor of 15.
  Every segment is also directly labelled, so identity never rests on colour.
- 5 new Rust tests for the parts that live in Rust rather than SQL.

### Deliberately absent, not approximated

Conversion rate, cart abandonment, show-up rate and on-sale velocity all need
data from the marketplace's side of the transaction. This app does not have it
and will not invent it.

## 2.20.0 - Deletions travel, the merge keeps a record, the AI has a price tag, and Drive has yesterday

**Migration 028. No new dependency, no business logic touched.** SYN-5/6/7/8.

### SYN-5 - Tombstones (migration 028)

The last hole in the merge, and marko named it: delete an order on the Mac and
it stays on the PC - then the next merge, seeing a record the Mac "has never
seen", copies it straight back. The deletion undoes itself.

A merge cannot notice the absence, because **a row that is merely not there is
indistinguishable from one that has not arrived yet.** Absence carries no
information. So the deletion has to leave something behind: `deleted_rows`,
written by an AFTER DELETE trigger on all 17 syncable tables.

- **Verified against real SQLite, not assumed:** an AFTER DELETE trigger DOES
  fire for rows removed by `ON DELETE CASCADE`, with or without
  `recursive_triggers`. That matters - deleting an order cascades to its
  tickets, and without it those tickets would leave no tombstone.
- Deletions are applied **children first** (reverse merge order), because
  `tickets.event_id` is `ON DELETE RESTRICT` and an event cannot go while a
  ticket holds it. A deletion the schema still refuses is reported, never
  forced, and never abandons the rest of the merge.
- Tombstones travel too, so a third copy cannot resurrect what was removed.
- **A tombstone beats an edit.** No per-row history exists to decide otherwise,
  and the alternative is resurrecting a record its owner deliberately removed.
- Probed against two real databases across 4 scenarios (9 assertions) before
  any of it was trusted, then covered by 3 Rust tests.

### SYN-6 - What came from the other computer

The merge already reported what it did; the report left with the toast. Now the
last 20 are kept - per table, with removals - and the sync card shows the last
three. JSON in `app_settings` rather than a table: nothing queries it, and
`app_settings` is a bookkeeping table, so writing the log cannot itself look
like data that needs syncing.

### SYN-7 - What the AI costs

The Messages API reports token usage on **every** response and this app was
throwing it away. Each screenshot import now counts towards a monthly total,
with an estimate at Claude Opus 5's published list price - **$5 / $25 per
million tokens**, read off Anthropic's pricing page today rather than recalled -
in integer cents, in USD because that is what Anthropic bills. Shown in the
existing API-key card. This does not contradict that card's long-standing note
about not showing a balance: a balance needs an Admin key and has no endpoint;
these tokens are measured, not guessed.

### SYN-8 - Earlier versions in Google Drive

The restore points are all on this machine, and the one copy that isn't gets
overwritten by every sync. It turns out the history was already there: **Drive
keeps a revision of every upload**, and `revisions.list` accepts
`https://www.googleapis.com/auth/drive.file` - the narrow scope this app
already holds, verified against Google's own reference. So point-in-time
recovery from off the machine, retroactively, with no new storage, no new
service and no new consent screen. Restoring one goes through the same
`restore_database_impl` as every restore (validation, safety backup, rollback)
and then pushes the older version to the other machine too, so the rescue
travels. `keepForever` is deliberately not set - pinning every sync would
multiply Drive usage by the number of syncs, so this shows what Drive kept and
says so.

5 new tests (3 tombstone, 2 cost). 179 commands still match `api.ts` <-> `lib.rs`
in both directions.

## 2.19.0 - The boxes stop breaking when the window isn't full screen

**No schema change, no new dependency, no business logic touched.**

marko: *"ked neni dashboard na full screene tak ze nebudu bugovat okienka"* -
and it was measurable, not a matter of taste.

1. **`.summary-bar` is a grid, not `flex flex-wrap`.** With `flex-1` the cards
   that land on the LAST row stretch to fill it. Measured in a real browser
   before and after, at the app's minimum window width (1080px -> 832px of
   content) and at its default (1400px -> 1152px):
   - 6 cards at 832px, before: five at 157px and **one at 832px** - a single
     box across the whole row.
   - 9 cards at 1152px, before: seven at 154px and **two at 570px** - so Price
     Checker and Event Detail did it on a normal window, not just a small one.
   - After, both cases: every card the same width, tidy rows.
   - 6 cards at 1152px: 182px each, one row, **identical before and after** -
     the common case is untouched.
   Resizing used to re-flow that stretch continuously, which is the "skákanie".
2. **A long figure can no longer push the row off the page.** A grid item
   defaults to its content width, so `StatCard` gained `min-w-0` and a
   truncating value. The 9rem column floor is what keeps a 22px money figure
   readable at the minimum window width.
3. **12 CSV export/import commands and the restore-point listing moved off the
   main thread** - the same fix the sync family got in 2.17.0. A large export
   was reading files and querying whole tables on the thread that draws the
   window. The Price Checker scanner was deliberately left alone: it creates
   windows through the `AppHandle`, and its logic is protected.
4. **Buttons that could error out.** 2.18.0's one-sync-at-a-time guard meant
   pressing Sync while the timer happened to be uploading came back "a sync is
   already running" - which reads as a bug, not a queue. Every sync button now
   disables for a sync started anywhere, and the panel stops saying "Syncing"
   by itself instead of waiting to be navigated away from.
5. **A recorded conflict now has a button.** "Combine both" only existed inside
   the prompt that appears after a push is refused, so a conflict recorded by a
   merge showed the state and offered nothing to do about it.
6. **Two stacking mistakes of my own, from 2.17.0.** The busy pill was
   `fixed bottom-4 right-4` - the same corner as the toast stack at `z-[100]`,
   so every toast hid the one thing meant to say the app was busy. And the
   blocking overlay sat at `z-50`, below `ConfirmDialog`'s `z-[60]`, so a
   confirm dialog could be clicked into a database being merged underneath it.

Also checked and clean: all 175 Tauri commands match between `api.ts` and
`lib.rs` in both directions (a typo'd `invoke` name is a button that silently
does nothing), every `api.X()` the UI calls exists, every `<Icon*>` and every
`ui.tsx` import resolves, and no literal-union state mismatch remains - the
class that broke the build twice this week.

## 2.18.0 - Sync gets safer, faster and harder to confuse

**No schema change, no new dependency, no new module, no new screen.** Every
change lands in `cloud_sync.rs` / `cloud_merge.rs` and in the sync card that
already existed in Settings.

1. **One sync at a time.** A process-wide `SyncGuard` - the only guards before
   were per-screen (`Layout.tsx`'s timer ref, Settings' disabled buttons) and
   neither knew about the other. 2.17.0 made that genuinely concurrent by
   moving these commands off the main thread: two uploads racing decide the
   winner by whichever request finishes last, then record a `version` for a
   file the other already replaced. Every entry point takes it - push, pull,
   merge - and the timer answers `idle` instead of an error, because "nothing
   to do right now" is the truth when it fires every five minutes anyway. It
   releases on drop, including on an early `?` return and on a panic.
2. **Bounded retries.** A dropped connection, a 429 or a 503 gets two more
   goes with 1s and 3s between. A 401/403/404 or an `invalid_grant` is
   returned immediately - those are answers, not hiccups, and retrying them
   makes the user wait longer for the same message. Never endless. The pause
   is only safe because 2.17.0 moved these commands off the main thread.
3. **Pointless uploads are gone.** The app records a hash of the bytes it last
   uploaded, so a database that was WRITTEN TO but does not actually DIFFER
   from Drive's copy is no longer shipped across. Running the migrations on
   launch marked it dirty, so every app update used to push several megabytes
   Drive already had, byte for byte. A download records the same hash, so a
   freshly pulled database is not immediately pushed back. Taking the snapshot
   is the cheap half of a push; this skips the slow half. The hash is FNV-1a
   plus the byte length, no dependency, and it fails in the safe direction:
   the same data laid out differently reads as "changed" (one extra upload),
   while the opposite needs a 1-in-2^64 collision.
4. **One short word for the state.** `synced` / `syncing` / `localChanges` /
   `cloudChanges` / `conflict` / `offline` / `failed`, decided by a pure
   `summarize_state` so the panel can never drift from what the timer decided,
   shown with the last successful sync and one readable sentence for the last
   failure. In the existing card - no Sync Center, no new tab.
5. **A conflict the merge could not settle now persists.** Skipped rows or
   `legacy-N` identity clashes set a conflict flag that only a clean merge
   clears - an ordinary push no longer hides it behind a green tick while the
   two machines still disagree about those records.

15 new tests: the guard (including that it frees on drop), the retry
classification both ways, the retry limit, the hash, all eight states, and the
error shortener.

## 2.17.0 - The window stops freezing while data moves

**No schema change, no new dependency.**

marko: *"su tam nejake nereagujuce spravy, nejake dlhe nacitavanie"*. Not a slow
query - a threading mistake, and it was in my own code.

1. **Tauri runs a SYNCHRONOUS command on the main thread.** Verified against
   the v2 docs, not memory: *"Commands without the async keyword are executed on
   the main thread unless defined with `#[tauri::command(async)]`."* Every
   Cloud Sync command was synchronous, so pushing a multi-megabyte database
   over the internet blocked the event loop and the OS drew "Not responding"
   over the window. Sheets sync and AI import never did this - they were
   already `async fn`, which is why only the sync family froze. 2.14.0's
   5-minute timer made it recurring and 2.16.0's merge made it long.
2. **16 commands that reach the network or move the whole database are now
   `#[tauri::command(async)]`**: the cloud sync/merge family, backup, restore,
   validate, `switch_active_database`, the Sheets connection tests, Google
   sign-in status and sign-out, currency conversion, the desktop-notification
   test. Their `State` argument gained the explicit `'_` lifetime that this
   codebase's already-async commands (`test_ntfy_notification`,
   `check_and_send_notifications`) already spell out.
3. **Something to look at while it works.** An app that is silently busy for
   eight seconds still looks broken even when it is perfectly responsive. A
   background upload gets a corner pill it can be ignored in; a download or a
   merge gets the screen, because both end in a restart or reload and a restart
   with no warning reads as a crash.
4. **The one silent failure the merge could still have, caught and reported.**
   Machines that drifted apart BEFORE migration 027 each counted up from the
   same place, so the Mac's 7th order and the PC's 7th order both call
   themselves `legacy-7`. The merge would see that identity already present,
   skip the arriving order as "already have it", and report nothing at all - a
   real order would simply never arrive. A shared identity whose `code` differs
   is now counted and named, together with the one manual sync that fixes it.
   Deliberately NOT repaired automatically: deciding which `legacy-7` is which
   is a human's call. 2 tests, and the detector was run against two real
   databases first - including the negative, so a genuinely shared record never
   trips it.

## 2.16.0 - The two machines add up instead of one replacing the other

**No schema change (027 did that), no new dependency.**

marko: *"ked ma jedna strana nieco ine a druha a das sync tak sa to zachova len
z jednej strany"*. Correct, and by design - push and pull move the whole
database FILE, so whichever direction runs, one machine's copy replaces the
other's. `commands/cloud_merge.rs` makes them add up.

1. **Records only one side has are copied in; records both sides have are left
   exactly as they are here.** So a merge cannot lose anything. It deliberately
   does NOT carry an edit or a delete across: an insert has one obvious correct
   outcome, an edit has two plausible ones, and guessing between them is the
   data loss this was written to end. Deletes need tombstones (a row that
   merely "isn't there" is indistinguishable from one that hasn't arrived yet)
   and that is its own migration.
2. **Ids are rewritten on the way in.** The other machine's order #12 becomes a
   different number here, so every foreign key is translated remote id ->
   `uid` -> local id, parents before children.
3. **Three collisions found by running the real thing, not by reading it.** The
   algorithm was executed against two actual SQLite databases before any of it
   was trusted:
   - **`code` is UNIQUE and both machines mint the same ones** from their own
     `counters` row, so `ORD-000007` exists twice. Arriving records whose code
     is taken get the next free one, and the count is reported.
   - **A lookup matched by name would have silently dropped its orders.**
     "Ticketmaster" typed on both machines is linked, not duplicated - which
     means this machine holds no row carrying the other's `uid`, so every
     arriving order pointing at it looked like an orphan. `translate_id` falls
     back to the name for exactly this reason.
   - **An arriving code from ahead poisoned the counter.** `ORD-000009`
     arriving while this machine's counter reads 5 breaks nothing that day -
     and then order creation starts failing on a UNIQUE constraint once the
     counter climbs to 9, with nothing on screen connecting it to a sync.
     Counters are now dragged up to the highest code present, before and after
     each coded table.
4. **Anything that still clashes is skipped, counted and named** (the same
   ticket sold on both machines), never forced and never allowed to abandon the
   rest of the merge. The whole thing runs in one transaction on top of a
   safety backup, which appears in Settings -> Data's restore points.
5. **Automatic sync no longer asks.** `decide_auto`'s two manual cases - both
   sides changed, and a machine that has never synced finding data in Drive -
   now answer `merge` instead of `ask`. Merging at launch reloads the page
   rather than relaunching the app: rows were added, the database file was not
   swapped. Mid-session it stays a banner, same rule as pull. Settings' old
   Take theirs / Keep mine prompt now leads with **Combine both**.

8 tests in the new module, all of them a scenario that was first run for real
against two databases: the platform-name drop and the counter poisoning are
both in there.

*Build fix, same version:* `syncBusy` is a fixed set of strings (`up`, `down`,
`toggle`) and the new Combine-both handler set it to `"merge"`, which `tsc -b`
rejected before the Rust ever compiled. The union now carries `"merge"` -
deliberately still a closed union rather than `string`, because every button in
that card disables itself on `syncBusy !== null`, so a missing value should stay
a compile error rather than a button that stays live mid-operation.

## 2.15.0 - Migration 027: the identity a merge is built on

**Schema change (migration 027), no new dependency, no behaviour change yet.**

marko chose merge-over-Drive: the two machines should combine what each has
instead of one overwriting the other. Merging needs one thing that did not
exist - a way to say "this row here and that row there are the same row".
Every primary key in this app is a per-machine `INTEGER AUTOINCREMENT`, so the
Mac and the PC both mint id 5 for two different orders. That is the exact
reason 2.12.0 shipped whole-file sync instead of a merge.

1. **`uid` on the 17 tables that can travel between machines** - events,
   orders, tickets, sales, pulls, pulls_received, ticket_listings,
   event_marketplace_links, payments, finance_entries, accounts, transfers,
   recurring_expenses, platforms, suppliers, event_categories,
   finance_categories. Not on `marketplaces` (seeded identically by these same
   migrations), not on the price-checker/`market_*` tables (per-machine scan
   history), not on the bookkeeping tables (the same list `is_bookkeeping_table`
   already keeps out of the sync dirty-flag).
2. **Existing rows get `legacy-<id>`; new rows get a random 128-bit uid.**
   Both databases descend from the same file, so a row that exists on both has
   the same id on both and therefore ends up with the same uid on both -
   without the machines talking. Shared history becomes shared identity for
   free.
3. **The uid is assigned by a trigger, not by Rust.** No existing INSERT
   changed - not the order writer, not the CSV import, not Sheets sync, not the
   AI import - and an insert site added later gets one automatically. The
   trigger fires only `WHEN NEW.uid IS NULL`, so a row arriving from the other
   machine keeps the identity it came with.
4. **One last whole-file sync is needed after both machines update.** If they
   had diverged first, their post-divergence rows carry colliding `legacy-N`
   uids. Push from whichever machine is current, pull on the other, once.
   After that hand-off nothing has to be overwritten again. Not automated on
   purpose: picking which side is "current" is the one judgement no timer may
   make.

Nothing reads `uid` yet - the merge engine itself is the next step. 5 tests.

5. **Dashboard: "This period vs. previous" (DSH-L), marko's pick, built wider
   than the mock he chose from.** The six KPI cards already carry this period's
   figure and a small "+62% vs. previous" label; what they never show is WHAT
   it is being compared to - the previous period is computed, sent, and thrown
   away after producing one adjective. This puts both columns side by side, and
   adds the line no card has: **profit per ticket**, which is the whole
   difference between selling more and selling better (revenue and ticket count
   can both rise while it falls). A table rather than the paired bars in the
   preview - the row above is already two cards of bars, and what was missing
   here was the numbers, not another shape. Cost is deliberately uncoloured,
   the same call `trendColored={false}` already makes on its card. No backend
   change: `previousPeriod` has been sent since 2.0.47, and every figure goes
   through the same `computeTrend`/`computeTrendPoints` the cards use, so a
   number here cannot disagree with the card above it. A range with nothing
   before it ("All time", a custom range with no start) says so in one line
   instead of rendering six dashes.

## 2.14.0 - Cloud sync runs by itself, both directions

**No schema change, no migration (next new one is still 027), no new
dependency** - `rusqlite` gains its `hooks` feature flag, which is the same
crate already in use, not a new one.

1. **The hand-off no longer needs a click.** With sync on, unsent work goes up
   every few minutes and anything new comes down when the app opens. marko asked
   for exactly this ("obe smery automaticky"), which retires 2.12.0's "nothing
   syncs automatically" rule - see `PROTECTED_AREAS.md`'s new 2.14.0 section for
   the rules that replace it.
2. **The new command decides and does nothing.** `cloud_sync_auto` makes one
   Drive metadata request and returns a verdict; `Layout.tsx` acts on it by
   calling the same `cloud_sync_push` / `cloud_sync_pull` the Settings buttons
   call. No second destructive path: every download still runs through
   `restore_database_impl`'s validation, safety backup and rollback. The policy
   itself is `decide_auto`, a pure function with the whole table on one screen
   and 11 tests over it.
3. **Two cases stay manual, on purpose.** Both machines changed since the last
   sync, and a machine that has never synced finding data already in Drive.
   Whole-file sync must pick a winner and neither case has an answer that isn't
   a guess, so both stop and ask. An automatic push never passes `force`, and an
   unreachable Drive is never read as "unchanged" - it does nothing and waits.
4. **Deleting something now counts as a change.** "Has this machine changed
   anything?" is answered by SQLite's own `update_hook` instead of a scan over
   `updated_at`: 17 of the 29 tables have no such column (`transfers` and every
   lookup list among them) and no table records a DELETE at all, so a timestamp
   scan would have missed whole classes of edit and let them be overwritten. The
   hook ignores the app's own bookkeeping tables, so the 30-minute notification
   tick can never trigger an upload.
5. **A machine closed before it could push still knows.** The flag is persisted
   to `app_settings`, written through on every check, on exit (`try_lock`, never
   blocking a close) and when accounts switch - so unsent work is not silently
   pulled over on the next launch.
6. **Restore points** (Settings &rarr; Data). marko asked for a way back "ak by
   ten sync nebol spravny". These files are not new - every destructive restore,
   cloud-sync downloads included, has always taken one first - but the only
   place their path ever appeared was a toast that disappears. `list_restore_points`
   reads both folders they land in (sync-down puts its own next to the active
   database, manual restore in `safety-backups/`), newest first, capped at 20.
   Read-only: it creates nothing and deletes nothing, and restoring one goes
   through the ordinary `restore_database`, which validates, takes its own
   backup first and rolls back - so undoing a bad sync is itself undoable.
7. **Dashboard: the two cards are one row again.** `items-start` let each end at
   its own content height, leaving a step; grid items stretch by default, and
   the platform card now fills the extra height from the inside (`flex-1` list)
   instead of showing dead space under its last row.

## 2.13.4 - One chart, profit back in the row, a scanner for Pulls

**No schema change, no migration (next new one is still 027), no dependency
change.**

1. **Profit rejoins the card row.** 2.13.2 lifted it out as a headline above
   the others; marko wanted all six on one line. It is a card again, just a
   louder one - brand-tinted border, larger figure, margin and ROI as its
   sub-line. Every card narrowed so six fit without wrapping.
2. **The second chart is gone.** Every series it could draw came from the same
   sales, so revenue and ticket count traced the same curve - the second frame
   repeated the first rather than adding to it. **Sales by platform** takes
   that slot: already on this tab below the fold, already the same
   `data.salesByPlatform` at the same period scope, and it answers *where* the
   money came from, which a line over time cannot. The **Cost** metric added in
   2.13.2 stays on the remaining chart.
3. **Pulls gets the screenshot scanner** Events, Orders and Sales already had.
   A new `"pull"` kind for the AI import fills event, date, section, row and
   seats off an image, reusing the `ticketGroups` seat-range expansion the
   `"order"` kind already describes. It deliberately does **not** read the
   buyer name or the pull fee: neither is ever printed on a ticket, and asking
   for either would invite exactly the guessing the feature forbids.

**Not built here:** no toolchain on the machine this was written on.

## 2.13.2 - Separate cards again, editable order prices, two charts

marko's own list after running 2.13.1. **No schema change, no migration (next
new one is still 027), no dependency change.**

1. **Back to one card per figure.** 2.13.1 put every summary figure in a
   single bordered bar; held up against the Attention Center row, marko wanted
   separate boxes. `StatCard` is a card again and `.summary-bar` is now just
   the row that holds them - one component change plus one CSS line, so all 13
   wrappers, Ticket Center's filters and the Calendar tiles followed without
   being touched. `SummaryStat` survives from 2.13.1 for Sales' own results
   strip, which genuinely is one line of text.
2. **An order's purchase price is editable after creation.** Unit price, fees
   and other costs, re-split across the order's tickets with the exact
   `allocate_cents` the create path uses - **including tickets already sold,
   which does retroactively change the profit reported on those sales.**
   marko's explicit choice over blocking the edit; the dialog says so whenever
   the order has sold tickets. 5 new tests.
3. **Fixed before it shipped**: the first version of that form used
   `parseFloat`, and `parseFloat("12,50")` silently returns `12`. It now uses
   `decimalStringToCents`, which the app already had and which handles the
   comma marko actually types. An empty unit price errors instead of zeroing
   the order.
4. **Fixed**: `orders_sheet_sync.rs` also builds an `OrderEditInput`, which the
   new required fields broke. It now reads the three price columns out of the
   database and passes them through, the same way it already did for
   `supplier_id` and `payment_status` - the sheet does not carry a price, and
   filling them with 0 would have zeroed every synced order's price.
5. **Pulls**: the Delete / New Pull buttons sit on the filter row instead of a
   right-aligned row of their own, which had left a band of empty space.
6. **Dashboard: two charts instead of one wide one**, each with its own metric
   switch rather than one being chosen for him. A fourth metric, **Cost**,
   joins Profit / Revenue / Sales - `cogs_cents` has been on every bucket since
   1.6.0 and was the one real series never offered. The headline figure reads
   the same field its line draws (`cogs`, not `total cost`), so the number and
   the chart cannot disagree.

**Not built here:** no toolchain on the machine this was written on. 2.13.2's
first build failed on point 4 above and nothing was ever published under that
number, so it is reused rather than bumped - same reasoning as the cancelled
2.4.0 direction.

## 2.13.1 - One summary style, everywhere

marko ran 2.13.0, pointed at the summary strip `Sales.tsx` has had since
1.9.0 - one border around the whole row, grey label, value beside it - and
asked for exactly that shape on every screen. **No backend change, no schema
change, no migration (next new one is still 027), no dependency change.**

1. **`SummaryStat` moved from `Sales.tsx` into `ui.tsx`** and is now the one
   implementation of a summary figure in the app. `StatCard` is a thin
   wrapper over it (trend and `sub` append inline), so its ~50 call sites
   follow automatically and Sales stops having its own private copy.
2. **`.stat-chip` became `.summary-bar`.** 2.13.0's chips were lighter than
   the old cards but still N bordered objects above a table; this is one
   border around the row. The 13 wrappers converted with it - and they no
   longer carry their own `mb-*`, since `.summary-bar` brings its own.
3. **Ticket Center's four filter cards are segments of that bar.** They were
   the largest thing on a screen whose point is the table underneath. Still
   buttons, still the filter, same counts. Their subtext moved to a `title`
   tooltip - a strip has no second line, and that text explained the count
   rather than naming it.
4. **Calendar's Today / Tomorrow / Next 7 days / Overdue** got the same
   treatment. Tiles with nothing to open now render as plain text instead of
   dead buttons.
5. **Dashboard's profit headline dropped 34px → 26px.** Still the loudest
   number on the page; no longer shouting.
6. **Sync is one button.** It works out the direction itself: if the Drive
   copy has not moved since this machine last synced, it pushes. If it has,
   both sides may hold work and nothing in the app knows whether the local
   side is also dirty - so it stops and asks (*Take theirs* / *Keep mine*)
   rather than guessing. The explicit Sync up / Sync down pair is still there
   under *Choose direction myself*.
7. **The app notices by itself.** `Layout` calls `cloud_sync_status` once on
   open and shows a dismissible bar when the other computer has newer data.
   It **checks and tells - it never syncs**; nothing moves without a click,
   so local-first still holds. This amends 2.12.0's "no startup sync" note in
   `PROTECTED_AREAS.md`, at marko's explicit request.

**Not built here:** no Node or Rust toolchain on the machine this was written
on, so nothing was compiled or tested - the first build is the first check.

## 2.13.0 - Visual redesign across seven screens, plus one balance fix

marko reviewed named alternatives one screen at a time and picked each of
these himself. **No schema change, no migration (next new one is still 027),
no dependency change, no change to refund/resell, `batch_id` or money
handling.** One backend change, unrelated to the redesign, is listed last.

1. **`StatCard` is a chip, not a bordered box** (`ui.tsx`, `index.css`). The
   summary row sat above ten screens at the same visual weight as the table
   underneath it and read as furniture. Changed on the **component**, so all
   ~50 call sites move together and the app keeps ONE summary style - which
   also means `EventDetail`, a screen this round never reviewed, changed with
   it. The callers' `grid` wrappers became `flex flex-wrap gap-2` (12 sites):
   a chip inside a grid cell stretches to the column and stops reading as a
   chip. Trend and `sub` are kept inline rather than dropped.
2. **Sidebar** (`Layout.tsx`): tighter rows, two quiet section headings, and
   the Tickets group given its own surface (the 2.6.0 hairline guide rail and
   the deep indent both go - the card is the grouping cue now). **No counts**,
   deliberately: `Layout` fetches nothing today and marko chose to keep the
   navigation pure frontend rather than add a command to feed numbers.
3. **Ticket Center** (`TicketCenter.tsx`): the `Completed` column mixed a
   stock fact ("Not Sold"), a payment fact ("Not Paid") and a count ("2
   Pending") under one heading, so it had no single meaning and a row never
   said why it was listed. Replaced by a **Needs** column built from the same
   `matchesCategory` predicate the four filter tiles already count with, so
   tiles and rows cannot disagree. Rows now sort **soonest event first**;
   orders with no event date sort last.
4. **Settings** (`Settings.tsx`): section **tabs** instead of six door-cards.
   The `settings/:section` route already existed, so every deep link still
   lands where it did; `/settings` with no section now opens the first tab.
5. **Dashboard** (`Dashboard.tsx`): profit becomes the headline figure with
   margin and ROI as its own sub-line; the other five stay as chips. Six
   identically-sized boxes claimed all six mattered equally.
6. **Finance** (`finance/Overview.tsx`): balance and what you are owed split
   into their own band **above** income/expenses/net. A stock and a flow are
   not the same kind of number, and the period filter only moves one of them.
7. **Calendar** (`Calendar.tsx`): the day detail is a **column beside the
   month grid** instead of a modal, so stepping through days no longer hides
   the grid. Empty days became selectable ("nothing on this day" is an
   answer). Week/Day/Agenda still use the modal - they have no grid to sit
   beside.
8. **Price Checker** (`PriceChecker.tsx`): the event overview is a **light
   table** instead of a card each - link state, listing count, scan age only.
   The per-marketplace breakdown moved behind the click that already opened
   the event (`onOpen={setEventId}` was there before this).
9. **Fixed (backend, separate from the redesign)**: `ACCOUNT_SELECT` had **no
   date filter at all**, so an entry dated in the future was already
   subtracted from a figure labelled *current balance*. It now cuts off at
   `date('now','localtime')`, matching `finance_forecast::eur_balance_as_of`,
   whose own test states the rule this query was breaking: *a future-dated
   entry must not already be folded into 'current' balance*. This is what made
   Finance Overview print two different "current balance" figures a few
   hundred pixels apart. **4 new tests** pin it, all using far-future/far-past
   dates so they never go stale. The cut-off is SQL's own `date()` rather than
   a `today` parameter - see the reasoning on `ACCOUNT_SELECT` before changing
   it.

**Not built in this session:** nothing was compiled or tested here (no Node,
no Rust toolchain on that machine) - the first CI run is the first real check.

## 2.12.1 - Three loose ends from 2.12.0

Cloud Sync is confirmed working end to end on both machines. These are the
follow-ups that were owed, not new features. **No schema change, no migration
(next new one is still 027), no business logic change.**

1. **Fixed**: the `tauri_plugin_deep_link::DeepLinkExt` import now carries the
   same `cfg(all(desktop, debug_assertions))` as the only call that uses it,
   so release builds stop warning. **Deleting it** - the obvious "fix" for an
   unused import - **would have broken `cargo tauri dev`**, where that
   dev-only deep-link self-registration is the whole reason the trait is in
   scope.
2. **Fixed**: the Drive 403 message. That one status covers two very different
   causes and the old text only named the less likely one ("sign in again").
   It now leads with the common case - the Google Drive API not being switched
   on for the OAuth client's Cloud project, which is a one-time, project-wide
   step that applies to every user of the build, not per person.
3. **Added**: Google's own 403 body carries the exact Cloud Console URL for
   that step, so the sync card now extracts it and offers an **Open Google
   settings** button instead of leaving it to be copied out of a toast that
   has already disappeared.
4. **Fixed**: `RELEASE.md`'s macOS instructions said right-click → Open, which
   Apple removed in macOS Sequoia (15). The working path is System Settings →
   Privacy & Security → Open Anyway.

## 2.12.0 - Cloud Sync: one database, two computers

marko works on a Windows PC and a Mac and wants what he writes on one to show
up on the other. **No schema change, no migration (next new one is still 027),
no business logic change, no new dependency, no server.**

1. **Added**: `commands/cloud_sync.rs` - keeps one database snapshot in
   marko's **own Google Drive** and syncs it whole-file, one direction at a
   time. "Sync up" uploads this machine; "Sync down" downloads and restores
   the other.
2. **Deliberately not built**: row-level merging. Every primary key here is a
   per-machine `INTEGER AUTOINCREMENT`, so two machines both mint id 5, and
   invariants like `insert_order_with_tickets`' exact-cent cost split and
   `refund_sale_impl`'s one-way transition have no correct automatic merge. A
   real merge engine is a months-long rebuild, not this release.
3. **Added**: a lost-update guard. Every upload records the Drive file version
   it wrote; the next upload re-checks it. If the other machine pushed since,
   the upload is **refused** and the UI offers an explicit "Overwrite anyway"
   decision instead of silently winning.
4. **Reused, not rebuilt**: downloads go through
   `backup::restore_database_impl`, inheriting its validation, automatic
   safety backup and automatic rollback - the safety backup path is returned
   and shown. Uploads use the SQLite Online Backup API via the new
   `backup::snapshot_db_to`, extracted from `create_safety_backup` so both
   callers share one implementation.
5. **Changed**: `OAUTH_SCOPE` gains `drive.file` - the narrowest scope that
   works, granting access only to files this app created and never to the rest
   of the Drive. **Everyone has to sign in with Google once more**: an
   existing refresh token was issued against the old scope set.
6. **Added**: a "Sync between your computers" card in Settings → Data, above
   Backup/Restore, since it is the same concern made automatic.
7. **Not changed**: nothing runs on a timer or at startup. Sync is off until
   switched on and every sync is a click. With sync off, or offline, the app
   is exactly as local-first as before.

5 new Rust unit tests.

**Build fix applied before this ever shipped**: the first attempt failed to
compile on both platforms with `no method named 'query' found for
reqwest::blocking::RequestBuilder`. `RequestBuilder::query` needs a
`serde_urlencoded` path that this crate's reqwest 0.13 feature set does not
enable, and no other module in this app had ever used it. Every URL is now
built the way `google_sheets.rs` has always built them - `format!` plus
`utf8_percent_encode` for anything dynamic. The download also streams
straight to disk via `std::io::copy` instead of buffering the whole database
in memory.

**Not verified by a build, and no Drive call has ever run.** No Node.js or
Rust toolchain on the machine this was implemented on. The Drive request
shapes are written from the Drive v3 API docs - they are the first thing to
check if sync misbehaves.

## 2.11.1 - Fix the macOS build broken by 2.11.0

2.11.0 published a working **Windows** release, but its macOS leg failed.

1. **Fixed**: the six `APPLE_*` signing variables are removed from the release
   build step. 2.11.0 wired them in "ready for the day credentials exist" -
   but **a missing GitHub secret still defines the environment variable as an
   empty string**, and the Tauri bundler treats `APPLE_CERTIFICATE` being *set*
   as a request to sign. It ran `security import` with an empty certificate and
   aborted with "failed to import keychain certificate". They are now a
   commented block carrying an explicit warning that pasting them back without
   the matching secrets breaks the build again.
2. **Fixed**: `prepare-release` now checks whether a release exists before
   deleting it, instead of letting `gh release delete` exit non-zero on the
   normal first-attempt case and paint a red "failed" annotation on an
   otherwise healthy run.

**What the failed run actually proved**: the Rust `universal-apple-darwin`
build succeeded in 3m36s and the `.app` was already bundling. The universal
target works on CI - signing was the only thing that ever failed.

Nothing else changed: no application code, no business logic, no schema, no
migration (next new one is still 027), no dependency.

## 2.11.0 - macOS builds, verified updater manifest, in-app update centre

marko asked for a professional installer + auto-updater + release pipeline.
Most of it already existed and was kept; the real gap was macOS. **No business
logic change, no schema change, no migration (next new one is still 027), no
new dependency, no new cloud service, no update server.**

1. **Added**: macOS builds. `.github/workflows/build-windows.yml` is renamed
   to `release.yml` and now builds **both** platforms - the Windows NSIS
   `.exe` and a **universal** macOS `.dmg` (Apple Silicon + Intel from one
   download). `release.ps1`'s own "did the workflow survive the mirror" guard
   was renamed in the same change - those two must always agree.
2. **Fixed**: the "delete stale GitHub release" step moved into its own job
   that runs **before** the build matrix. With a matrix, the second runner
   would otherwise delete the release the first one had just published.
3. **Changed**: the release matrix runs one platform at a time
   (`max-parallel: 1`) - both legs publish to the same release and both
   rewrite `latest.json`, so serialising them removes the race.
4. **Added**: a `verify-release` job asserting the finished release really has
   an `.exe`, a `.dmg` and a `latest.json`, that the manifest covers both
   platform families, and that every entry has a non-empty signature and url.
   A silently-wrong updater manifest is the failure nobody notices until users
   stop getting updates.
5. **Added**: a very small update status on the Dashboard. It reads the
   launch-time check result and links to Settings → Software; it does not run
   its own check and does not install anything, so there is still exactly one
   updater UI and one progress state.
6. **Added**: Settings → Software now shows Current version / Latest version /
   Last checked alongside the existing check-and-install flow and release
   notes.
7. **Changed**: the launch-time update check now also repeats every 6 hours
   for sessions that stay open - deliberately slow, no polling. Manual "Check
   for updates" is unaffected.
8. **Added**: `RELEASE.md` - the release process, every repository secret and
   what it does, the artifact list, and both the developer and user
   checklists. No secret values in it.

**Signing, stated honestly:** updater signing is configured and is what makes
in-app updates secure. **Windows code signing and macOS code signing/
notarization are NOT configured** - both need paid external credentials. The
workflow passes all six Apple variables through so adding the secrets is the
only remaining step, and nothing here fakes a certificate. Until then the
`.exe` shows a SmartScreen warning and the `.dmg` needs right-click → Open on
first launch.

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, and no macOS build has ever run - the first tagged
run of this workflow is itself the test.

## 2.10.0 - Price Checker event overview replaces the event dropdown

marko's own request: see every relevant event at once instead of picking one
from a dropdown. **UX only** - the scanner, parser, readers, market analysis
and history are untouched. **No schema change, no migration (next new one is
still 027), no new dependency.**

1. **Changed**: opening Price Checker now lists every **upcoming** event as a
   dense card - name, date, venue/city, a per-marketplace row (Linked / No
   link, plus that marketplace's last check and listing count), and an
   event-level line with the newest listing count or "Not scanned yet".
2. **Added**: multi-select with checkboxes, "Select all" (scoped to what is
   currently visible, so a filtered list can't silently select what you can't
   see), "Clear selection", and a "Selected: N events" bar.
3. **Added**: local search over event name, venue and city, and four quick
   filters - All / Needs link / Not scanned / Scanned.
4. **Added**: one read-only backend command, `list_price_checker_overview`,
   answering the whole list in four flat queries instead of calling
   `get_price_checker_summary` once per event. It writes nothing, triggers no
   scan and makes no marketplace request.
5. **Deliberately not added**: a "Scan failed" state. A failed scan is never
   persisted anywhere in this app - `price_checks` has no status column, and a
   row only reaches it through the explicit review-then-save step, so by
   construction every stored check succeeded. The scanner's own error/blocked
   states live in memory for the life of one scanner window. A "Scan failed"
   badge would be inventing a state the database does not have.
6. **Check selected**: opens the first selected event's own flow and keeps the
   selection, so the rest are one click each. The scanner opens a real visible
   browser window marko drives himself, so no parallel sessions and no queue
   automation were invented.
7. **Added**: an "All events" back link, since the dropdown that used to be
   the way back is gone.
8. **Not changed**: parser, readers, DOM scanning, market calculations, tier
   grouping, section/row metadata, price history, Your Tickets. Tier/Level
   stays a market grouping; section/row stay metadata. No background
   monitoring, no polling, no scheduled scanning, no repricing.

8 new Rust unit tests (42 in `price_checker.rs` total).

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, so `cargo test --lib`, `cargo check --lib`,
`npx tsc -b` and `npm run build` could not be run.

## 2.9.0 - Price Checker accuracy fix, scan report, filters, CSV export

marko reported the scanner reading prices wrong. Diagnosis first, then the
smallest fix per cause. **No schema change, no migration (the next new one is
still 027), no new dependency, no new marketplace.**

1. **Fixed (root cause of the wrong prices)**: `price_checker_scan.js`'s
   generic text-node walker found money in one text node and then called
   `candidateFrom(parentElement)`, which re-parsed the parent's *entire* text
   and kept the **first** money match in it. On "Was $200 / Now $120" markup
   that stored the crossed-out old price; on a wrapper whose `aria-label`
   reads "Total incl. fees" it stored the fee-inclusive total. The matched
   money, and the text it was matched in, are now passed into
   `candidateFrom`.
2. **Added**: three price rejection rules that did not exist at all before -
   struck-through prices (`<s>/<del>/<strike>`, line-through class or computed
   style), money labelled total/subtotal/fee/service charge/delivery/tax/was/
   original/RRP, and money inside header/footer/nav/aside/cart/checkout/modal
   page chrome.
3. **Fixed (metadata leak)**: `findListingContainer` fell back to "three
   ancestors up" and `nearbyListingContext` then regexed that whole subtree,
   so section/row/quantity/tier could be read off a *neighbouring* listing or
   off page chrome. The container now reports whether it is confident;
   metadata is read from a tight scope when it is not, and the listing is
   flagged `incomplete` instead of being presented as a clean read.
4. **Fixed (dedup, both directions)**: a real marketplace listing id is now
   the whole cross-scan identity - it used to be one of seven fields
   *including the price*, so the same listing re-read after a scroll counted
   twice as soon as the price read differently. And `tier` joined the fallback
   key, whose absence merged two listings that differed only by tier and
   deleted a real one. Without an id the price stays in the key on purpose.
5. **Fixed (currency blending)**: `compute_scan_stats` averaged across every
   listing regardless of currency and labelled the blended result with
   whichever currency came first. It now computes inside the largest
   single-currency group only and reports how many listings were excluded -
   which also makes it agree with `price_checker_analysis`, which has always
   partitioned by currency correctly.
6. **Added**: a Found / Accepted / Skipped / Duplicates scan summary with
   per-reason skip counts.
7. **Added**: six plain result filters (marketplace, tier, currency, min/max
   price, complete vs incomplete) - not a filter builder.
8. **Added**: Export CSV, reusing the same `plugin-dialog` `save()` + Rust
   writer every other export in this app already uses.
9. **Not changed**: the manual Visible Scanner workflow, Tier/Level as a
   grouping (never a pricing input - no section/row/seat pricing, no price
   suggestions), Your Tickets comparison, history, refund/resell, `batch_id`,
   money/integer cents, Orders/Tickets/Sales/Listings/Finance/Fulfillment/
   Attention/Calendar/Google Sheets. No background monitor, no scheduled scan,
   no polling, no CAPTCHA bypass.

15 new Rust unit tests (43 in `price_checker_scanner.rs` total).

**Not verified by a build, and the DOM fixes are not verified against a live
page.** No Node.js and no Rust toolchain on the machine this was implemented
on, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and `npm run
build` could not be run, and the injected browser script cannot be executed
here at all. The live marketplaces have never been reachable from this
sandbox either (the same limitation the 2.1.9 script has always carried), so
every DOM-level fix is reasoned from the code and must be confirmed against a
real listings page.

## 2.8.0 - Calendar redesign + advanced calendar workflow

marko asked to make the TIQR Operations Calendar one of the app's main work
screens. **No schema change, no migration (the next new one is still 027), no
new index, no new dependency.** `commands/calendar.rs` stays what it has
always been: a read-only aggregator with no write path.

1. **Added**: four views instead of two - **Month, Week, Day, Agenda**. All
   four share one data path (`get_calendar` over a date range), one filter
   state and one search box; only the range and the layout differ, so
   switching period still fetches exactly one window.
2. **Added**: two genuinely new date sources, **`finance`**
   (`finance_entries.entry_date`) and **`recurring`**
   (`recurring_expenses.next_date`). Both were re-derived from the live
   schema and command code rather than trusting the 2.5.0 note, exactly as
   `PROTECTED_AREAS.md` instructs.
3. **Still not added**: payouts, payments, fulfillment. Re-checked this
   release - there is no payout entity or date anywhere, migration 007's
   `payments` table still has zero live SQL in any command module, and
   `tickets.delivery_status` still has no date column. What the 2.5.0 pass
   never checked was Finance, which is where this app's real dated money
   lives - so it is added under its own honest name rather than rebranded as
   a "payout". There is a standing test that fails if an invented category
   ever reaches the calendar.
4. **Added**: a real **Overdue** count. `recurring_expenses.next_date` is the
   only genuine due date in the app; the rule is the same one Finance's own
   Accounts tab applies (active template, `next_date` before today), and
   paused templates are excluded because their `next_date` is frozen and not
   actionable.
5. **Added**: a Today / Tomorrow / Next 7 days / Overdue summary strip, a Day
   Detail with a per-kind summary panel, per-day workload bars (three muted
   segments, deliberately not a color scale), event countdowns derived only
   from the event's own date, calendar-local search (dims non-matches in the
   grid, filters the lists, can jump to the first match), and filter chips
   that only appear for kinds actually present in the loaded range.
6. **Deliberately not built**: a time-of-day axis. Every date this app stores
   is date-only, so Week is seven day columns rather than a faked 24-hour
   timetable, and Day groups by kind rather than by hour.
7. **Deliberately not built**: quick-add of tasks/reminders/notes. There is
   no task, note or reminder table anywhere in this schema, and marko was
   explicit that this release must not stand up a new task database.
8. **Changed**: the grid and the lists scroll inside themselves rather than
   growing the page, so the header, filters and summary strip stay put and
   there is only ever one scrollbar.
9. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment/Attention business
   logic, Price Checker, Google Sheets.

9 new Rust unit tests (23 in `calendar.rs` total).

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
Run all four before publishing the tag, and regenerate
`Cargo.lock`/`package-lock.json`.

## 2.7.0 - AI Import Assistant (screenshot -> pre-filled form, never straight to the database)

marko's own request: drop, paste (Ctrl+V) or upload a screenshot into the New
Event, New Order or New Sale form and have Claude read the structured data off
it. **No schema change, no migration (the next new one is still 027), no new
dependency.**

The rule the whole feature is built around, in his words: "AI NIKDY nesmie
priamo vytvoriť alebo meniť databázový záznam." The flow is image -> Claude ->
structured result -> review -> user confirms -> the EXISTING create form -> the
EXISTING create command -> DB.

1. **Added**: `src-tauri/src/commands/ai_import.rs` - one command,
   `analyze_import_image`. It takes no `AppState`, opens no `Connection` and
   has no write path of any kind, so it cannot create or change a record even
   by mistake. Reuses `ai_categorize.rs`'s build-time embedded
   `ANTHROPIC_API_KEY` (the key never reaches the frontend) and its
   retry-once-on-a-transient-status policy.
2. **Added**: a strict per-kind JSON schema (`output_config.format`). Every
   extracted value is a plain string or `null`, each carries a
   `high`/`medium`/`low` confidence, and a field that isn't on the image comes
   back `null` rather than being filled in. `sanitize_result` then drops any
   field name the form has no slot for, keeps only the first of a duplicate,
   and collapses blank values to `null`.
3. **Added**: multiple ticket groups are returned separately and never merged.
   Because `OrderInput` carries one section/row/tier/price per order, the panel
   fills one group at a time and says so, instead of inventing a multi-group
   order shape the backend has never had.
4. **Added**: `src/components/AiImportPanel.tsx` - one compact shared panel,
   embedded in all three forms. Not a new page, route, tab or sidebar.
5. **Added**: `src/lib/aiImport.ts` - image validation, downscale to 1568px
   (only when needed, so a normal screenshot is sent untouched), an FNV-1a
   fingerprint, clipboard/drop extraction, an ISO-date guard and a lookup
   matcher.
6. **Changed**: `dragDropEnabled: false` on the main window
   (`src-tauri/tauri.conf.json`) so HTML drop events reach the webview -
   Tauri's default of `true` swallows them. Nothing in the app used Tauri's own
   drag-drop event.
7. **Cost control**: one analysis per explicit user action; a per-session image
   fingerprint cache so the same screenshot is never analyzed twice; editing an
   extracted field never re-calls; retry only on a click; no background or
   timed request anywhere.
8. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment logic, Price Checker,
   every existing create command and every existing validation rule. AI
   extraction cannot bypass validation - the same commands still run at save
   time on values the user has seen and confirmed.

9. **Fixed (same version, after the first release run failed)**:
   `release.ps1`'s `$CommitMsg` had literal double quotes in it, which Windows
   PowerShell 5.1 cannot pass to `git.exe` intact - `git commit` exited
   non-zero and the script stopped with "git commit failed". The quotes are
   gone and there is now a guard right before the commit that catches this
   with a real explanation instead of a git error. See `PROTECTED_AREAS.md`'s
   "2.7.0" entry.

**Needs the `ANTHROPIC_API_KEY` GitHub Actions secret** (already wired into
both build paths). Without it the panel reports "AI import isn't available in
this build" and nothing else changes.

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
The 28 new Rust unit tests are written but have never been executed. Run all
four before publishing the tag, and regenerate `Cargo.lock`/`package-lock.json`.

## 2.6.0 - Complete visual redesign (UI/UX only)

marko's own task: make TIQR Manager look and feel like a modern, premium
desktop app, explicitly with **no new features, no new workflow, no new
database systems**. Nothing in `src-tauri/` changed - the backend is
byte-for-byte identical to 2.5.2. No migration, no new dependency.

1. **Changed**: one shared design layer now defines the whole app's look -
   `tailwind.config.js` (retuned `slate` ramp so light and dark each get a
   real background -> surface -> line hierarchy instead of one being an
   inversion of the other; a 3-step `shadow-card`/`raised`/`overlay` scale;
   a tighter radius rhythm; a 120-180ms motion budget). The `brand` blue
   ramp is deliberately untouched.
2. **Changed**: `src/index.css` - base typography (tabular figures on every
   number in the app), restyled `.input`/`.label`/`.th`/`.td`/`.card`, new
   `.section-title`, `.field-invalid`, `.skeleton`, `.card-interactive`, and
   the new `.table-shell`/`.table-flush`/`.row-selected` table system.
   `.th-c-narrow`/`.td-c-narrow`'s measured 2.0.37 metrics are unchanged.
3. **Changed**: `src/components/ui.tsx` - every shared control restyled.
   `Button` gained an optional `size`; `Field` now turns its own control red
   on error; `Badge`/`InlineStatusSelect` share one status pattern with a
   leading dot; `StatCard` is shorter and quieter; `Modal`/`ConfirmDialog`/
   `EmptyState`/`ModalFooter`/`TabSwitcher` redesigned. The status
   vocabulary (which tones exist, which value maps to which) is unchanged.
4. **Added**: `Skeleton` and `TableSkeleton` in `ui.tsx`, plus
   `SEGMENTED_TRACK`/`segmentedItemClass` - the app's one tab/segmented
   pattern, now genuinely shared instead of hand-rolled per page.
5. **Changed**: `src/components/Layout.tsx` - sidebar redesigned (accent-bar
   active state, hairline section separation, restyled theme toggle and
   profile area). Same items, same order, same routes, same `w-48` width,
   same behaviour.
6. **Changed**: all 22 tables in the app moved onto the shared table shell -
   sticky opaque headers, internal scrolling instead of page scrolling, one
   hover treatment, one selected-row treatment. Column widths, `colgroup`
   percentages and the narrow-window breakpoint are untouched.
7. **Changed**: six list pages (Orders, Sales, Tickets, Events, Pulls,
   Ticket Center) now show a table skeleton while loading instead of a
   centred spinner.
8. **Changed**: `prefers-reduced-motion` now disables every transition and
   animation across the app.
9. **Not changed**: any business logic. The whole diff contains no `api.`
   call, no state/handler/effect, and no routing change - see
   `PROTECTED_AREAS.md`'s "2.6.0" entry.

**Not verified by a build.** This round was implemented on a machine with no
Node.js and no Rust toolchain, so `npx tsc -b`, `npm run build` and
`cargo check --lib` could NOT be run - marko chose to proceed on static
review rather than install a toolchain. Run all three before publishing the
tag, and regenerate `Cargo.lock`/`package-lock.json` (their version entries
were bumped by hand for the same reason).

## 2.5.2 - "Forgot password?" via a deep link into the app; Discord sign-in deferred

marko's own follow-up request after 2.5.1. No schema changes. Discord
sign-in was asked for too but is NOT built - see `PROTECTED_AREAS.md`'s
"2.5.2" entry for why (needs a Cloud Function + Firebase's paid Blaze plan,
which marko chose against for now).

1. **Added**: a real "Forgot password?" flow from Welcome.tsx's login form,
   using Firebase's own `sendPasswordResetEmail`/`verifyPasswordResetCode`/
   `confirmPasswordReset` - no new backend, no Cloud Functions, no Blaze
   plan.
2. **Added**: the emailed reset link now opens TIQR Manager directly
   (`handleCodeInApp: true` + a new `tiqrmanager://` custom URL scheme via
   `tauri-plugin-deep-link`) instead of a browser tab - marko's first choice
   (a typed short code) would have needed the same paid infrastructure as
   Discord above, so this was worked out with him directly as the
   alternative.
3. **Added**: `docs/reset-redirect.html`, a static hand-off page on the same
   GitHub Pages site `docs/privacy.html` already uses, forwarding Firebase's
   link into the app.
4. **Added**: `src/pages/ResetPassword.tsx` - verifies the incoming reset
   code and sets the new password.
5. **Needs one manual step**: add `biznismarko9-source.github.io` to Firebase
   Console -> Authentication -> Settings -> Authorized domains before this
   works end to end.

## 2.5.1 - Ticket Center rebuilt around orders, sidebar reorder, Calendar visual refresh

marko's own direct follow-up on the 2.5.0 release below, delivered as its
own round. No backend/schema changes. Packaging
(`REDESIGN-2.5.1-REPORT.md` + zip) was deferred past this round - marko
signaled more feedback was coming right after - and built once he
separately asked ("zabal to"; see `PROTECTED_AREAS.md`'s "2.5.1" entry).

1. **Changed**: Ticket Center moved out of Finance (was a subtab there for
   one version, 2.4.4) back to its own top-level page/route
   (`/ticket-center`).
2. **Rebuilt**: Ticket Center now lists ORDERS (via the same `api.listOrders`
   Orders.tsx already calls), not individual tickets/sale-batches - click an
   order to see and edit what's outstanding on each of its tickets on the
   existing Order Detail page. `TicketControlCenter.tsx` (2.4.3),
   `FulfillmentCenter.tsx` (2.2.12), and the `finance/TicketCenter.tsx`
   subtab shell are all deleted; 4 new quick-filter tiles (Needs
   attention/listing/payment/delivery) replace both pages' old filter/
   category systems.
3. **Changed**: sidebar top-level order now matches marko's exact list -
   Dashboard, Tickets, Price Checker, Pulls, Finance, Ticket Center,
   Calendar (Calendar moved from right after Dashboard to last).
4. **Changed**: Order Detail's "arrived from" Back link now also recognizes
   Ticket Center as an origin (`/ticket-center` -> "Back to ticket center").
5. **Visual refresh only**: the Calendar page (2.5.0) - a consistent accent
   color per entry kind across the grid/filters/modal/summary, severity
   shown as a ring/text-color layered on top instead of the only signal,
   weekend/today cell shading, and a weekday name in the Day Detail title.
   No data, hook, or navigation change.

## 2.5.0 - TIQR Operations Calendar

marko's own spec for a new cross-domain Month/Week calendar page, delivered
together with the 2.4.4 round below in this same release.

1. **New**: `/calendar` page - a Month/Week calendar aggregating every part
   of the app with a real date: events, orders, sales (grouped by batch,
   never one row per ticket), pulls, and Attention Center items. Today/
   Previous/Next navigation, a Day Detail view (click any day or its
   "+X more"), a client-side Filters row, and a "Today & next 7 days"
   summary card.
2. **New backend**: `commands/calendar.rs` (`get_calendar`, one command) +
   `CalendarFilters`/`CalendarEntry` models. No new migration, no new
   table - reuses `attention_center::get_attention_center_impl` and
   `sales::GROUP_KEY_EXPR` directly rather than re-deriving either.
3. **Deliberately NOT implemented**: payout, payment, and fulfillment
   calendar entries - none of the 3 has a real, reliably-existing date
   anywhere in this app today (see `PROTECTED_AREAS.md`'s new "2.5.0"
   entry for the full research). Nothing was invented to fill these in.
4. **New sidebar entry**: "Calendar", directly below Dashboard.
5. 14 new Rust tests (`commands/calendar.rs`), full suite green (1052
   tests); `npx tsc -b` and `npm run build` both clean.

## 2.4.4 - Ticket Center consolidation, sidebar regroup, theme toggle

marko's own request, a pure frontend/UX round (no backend, schema, or
migration changes at all) delivered before his separate 2.5.0 Calendar
spec.

1. **Merged**: Ticket Control Center (2.4.3) + Fulfillment Center (2.2.12),
   previously two standalone top-level sidebar pages, now live under
   Finance as one "Ticket Center" tab with two subtabs (Control Center,
   Fulfillment) - new `finance/TicketCenter.tsx`. `/control-center` and
   `/fulfillment` routes removed; both components reused unchanged
   internally aside from Control Center's own fixes below.
2. **Sidebar regrouped**: Events/Orders/Tickets/Sales/Inventory now sit
   under one collapsible "Tickets" entry instead of 5 flat rows - all 5
   routes themselves unchanged.
3. **New**: one-click light/dark toggle above the sidebar's profile widget,
   reusing the existing `useTheme()` hook. Settings -> Appearance (the old
   3-way Light/System/Dark picker) removed - moved, not duplicated.
4. **Fixed**: Ticket Control Center's sticky header had a translucent
   dark-mode background (`dark:bg-slate-800/60`) letting scrolled row text
   bleed through while scrolling - now fully opaque.
5. **Changed** (Ticket Control Center): "Ticket / Seats" column renamed to
   "Seats", now shows only the seat location (ticket code moved to a hover
   tooltip); Order cell now independently opens Order Detail on click.
6. **Changed** (Dashboard): "Sales by platform"'s internal scrollbar
   removed, replaced with the same slice + "Show N more" pattern the
   Activity tab's Recent cards already use.

`npx tsc -b` and `npm run build` both clean; no Rust changed, so
`cargo test --lib` is unaffected. Version bumped **2.4.3 -> 2.4.4**.

## 2.4.3 - Ticket Control Center

marko's own focused-task request: one central work screen to manage and
check tickets across every event at once, built entirely on top of the
existing tickets/listings/sales data - explicitly not a new parallel ticket
system.

1. **New**: `/control-center` page - sticky filters (Event, Date range,
   Tier/Level, Section, Row, Ticket status, Listing status, Sale status,
   Payment status, Delivery status, Marketplace), 8 quick filters (All/
   Unsold/Unlisted/Listed/Sold/Pending payment/Pending delivery/Refunded),
   search across ticket/order/event/section/row/marketplace/listing id-url,
   and a dense table (first in this app to own its own scroll instead of
   growing the whole page) over one new backend query,
   `list_control_center_tickets` (`commands/ticket_control_center.rs`). Row
   click opens the existing Sale Detail or Order Detail, whichever applies.
2. **Bulk actions - all existing mechanisms**: Section/Row/Tier/Seat/Listing
   price via the shared `BulkTicketEditBar` (now with a new Tier option,
   also benefiting Sale Detail/Order Detail); listing status via the
   existing `bulkUpdateTicketListingsStatus`; CSV export of the selection
   via the existing `exportTicketsCsvSelected`. No refund/resell bulk
   actions, per marko's explicit instruction.
3. **New, purely additive read signal**: `isRefunded` (an `EXISTS` check
   against `sales`) - lets the "Refunded" quick filter surface a
   refunded-and-not-yet-resold ticket, which the existing active-sale-only
   join can't otherwise distinguish from a never-sold one. Reads only; no
   refund/resell/money logic touched.
4. **Untouched**: refund/resell logic, `batch_id`, every money/cents column,
   Listings/Sales/Finance/Orders core - per marko's explicit "DÔLEŽITÉ" list.
   No new migration.

Full backend suite green: `cargo test --lib` (1038 passed, +12 over 2.4.2,
0 failed, 3 ignored), `cargo clippy --lib` clean of new warnings, `npx tsc
-b`, `npm run build` all clean. Version bumped **2.4.2 -> 2.4.3**.

## 2.4.2 - Live Market Monitor removed; Price Checker back to a manual tool

marko decided he does not want the 2.4.1 "Live Market Monitor" feature in
the app at all (*"TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC"*) and asked for
it removed entirely - no background/scheduled scanning, no automatic
monitoring, ever - with Price Checker returned to a purely manual tool.
Full reasoning in `PROTECTED_AREAS.md`'s new "2.4.2" entry.

1. **Removed entirely**: backend module `price_checker_monitor.rs` and its
   2 commands (`get_market_monitor_summary`, `list_market_snapshots`); both
   scan-result hooks into it from `price_checker_scanner.rs`; the
   `market_alert` Attention Center category (`attention_center.rs`'s
   `push_item` reverted to its original 2-shape key, Dashboard's 6th box
   and grid column removed); Auto Monitor (ON/OFF + 15m/30m/1h/3h/6h
   interval) and "Scan All" from `PriceChecker.tsx`; the Live Market
   Monitor panel and the Market History view/modal; every related Rust
   struct (`models.rs`) and TypeScript type (`types.ts`/`api.ts`).
2. **Price Checker unchanged**: event selection, marketplace URLs/source
   handling, the manual Visible Scanner, Market Analysis, tier/section
   grouping, price history, Your Tickets comparison - all pre-existing
   functionality, untouched. No redesign.
3. **Database**: migration `026_price_checker_market_monitor.sql` and its 4
   tables (`market_snapshots`, `market_snapshot_tiers`, `market_source_
   status`, `market_alerts`) were **kept in the schema, not deleted** - 2.4.1
   already shipped and marko's own local DB has already run this migration,
   so this codebase's forward-only migration rule means it can never be
   safely deleted or renumbered. Only the application code that read/wrote
   these tables was removed; no user data was touched. The next new
   migration is **027**, not a reused "026".
4. Two small judgment calls, flagged per this codebase's own "smallest
   consistent solution, flag on ambiguity" convention: "Scan All" was
   removed as in-scope (it existed purely to complement Auto Monitor, no
   standalone purpose without it); `price_checker_analysis.rs`'s
   `group_by_tier` was left `pub(crate)` rather than reverted to private
   (bumped in 2.4.1 for the now-deleted module to reuse - harmless residual,
   not worth an extra touch to that protected file for zero functional
   gain).

Full backend suite green: `cargo test --lib` (1026 passed, -32 removed with
the feature, 0 failed, 3 ignored), `npx tsc -b`, `npm run build` all clean.
Version bumped **2.4.1 -> 2.4.2** - removing a shipped feature still bumps
the version forward, never back to a number already used (same precedent as
2.3.0's revert shipping as 2.3.1) - see `PROJECT_STATE/CURRENT_STATE.md`'s
"## Version" section. See `PROTECTED_AREAS.md`'s new "2.4.2" entry before
ever touching migration 026, `group_by_tier`, or migration numbering again.

## 2.4.1 - Price Checker Live Market Monitor

Marko cancelled the "Live Event Intelligence" direction below outright
("Predchádzajúci nápad 'Live Event Intelligence' RUŠÍME ÚPLNE") and asked
for all online/live-market functionality to live directly inside Price
Checker instead: EVENT -> MARKETPLACE SOURCES -> SCAN -> SNAPSHOT -> HISTORY
-> CHANGE DETECTION -> MARKET ALERTS, built entirely on the already-shipped
Visible Scanner (2.1.9) and Market Analysis (2.2.0) - no CAPTCHA bypass, no
proxy rotation, no anti-bot workaround, and no automation beyond reading
whatever a human already has open in a real, visible window. Full reasoning
in `PROTECTED_AREAS.md`'s new "2.4.1 - Price Checker Live Market Monitor"
entry.

1. **New backend module `price_checker_monitor.rs`** - records a permanent,
   never-overwritten snapshot after every successful/partial scan
   (`market_snapshots`/`market_snapshot_tiers`, migration `026_price_
   checker_market_monitor.sql`), tracks each marketplace's connection status
   (`market_source_status`: not_connected/connected/success/failed), and
   diffs each new snapshot against the previous one - overall and per tier
   (never per section/row/seat) - to raise MARKET DROP / MARKET RISE / NEW
   SUPPLY / SUPPLY DROP alerts (`market_alerts`) at transparent, reused
   thresholds (5% price, 20% supply - the same constants Price Checker's own
   recommended-price and Inventory Intelligence's own outside-market logic
   already use elsewhere). A SOURCE FAILURE alert fires only on a genuine
   success-to-failure transition, never on the first-ever failure or on
   repeated consecutive ones - keeps this quiet instead of noisy.
2. **Auto Monitor** - an ON/OFF toggle with a 15m/30m/1h/3h/6h interval,
   scoped to one already-open Visible Scanner window per marketplace card;
   it is the identical "Scan Visible Prices" call the button already makes,
   fired on a schedule, never opening a window or reading a page on its own,
   and it turns itself off the moment that window closes. "Scan All" fires
   the same call once for every marketplace on the current event that
   already has a window open.
3. **Price Checker UI**: each marketplace card gained a Live Market Monitor
   panel - connection status, last successful scan (never cleared by a
   later failure - the app stays useful on cached data even fully offline),
   the latest snapshot's stats, Auto Monitor controls, and its recent Market
   Alerts; plus a "Market History" view of every saved snapshot.
4. **Dashboard Attention Center gained a 6th box, "LIVE MARKET ALERTS"**
   (`market_alert` category, the single most recent alert per event/
   marketplace) - named differently from marko's own literal spec wording
   ("MARKET ATTENTION") because that title was already taken by the
   existing `outside_market_price` box (2.2.11, an unrelated feature: your
   OWN listing prices vs. the market). Clicking a row jumps straight to
   Price Checker at that event and marketplace (scrolled into view and
   briefly highlighted) - no new separate dashboard.

32 new backend unit tests (27 in `price_checker_monitor.rs`, 5 in
`attention_center.rs`). Full suite green: `cargo test --lib` (1058 passed, 0
failed, 3 ignored), `npx tsc -b`, `npm run build`. See `PROTECTED_AREAS.md`'s
new "2.4.1 - Price Checker Live Market Monitor" entry before touching any of
this again, and the entry directly below for why reusing the version number
"2.4.0" was verified safe before this release ultimately moved one step
further to **2.4.1** instead (a plain filename-collision reason, not an
auto-updater one - see `PROJECT_STATE/CURRENT_STATE.md`'s "## Version"
section for the full story).

## 2.4.0 (pre-release direction, never shipped) - Live Event Intelligence Foundation - REVERTED, see entry above

marko's next spec after 2.3.5: an Event can now optionally carry a
CONFIRMED online identity on exactly 3 marketplaces - Viagogo, Vivid Seats,
Ticombo. Foundation work only - no pricing logic, no changes to the
existing Price Checker or its scanner. Full reasoning in
`PROTECTED_AREAS.md`'s new "2.4.0 (pre-release direction)" entry.

**Kept as history only (this file is append-only) - marko reviewed this
build and cancelled the whole direction outright** ("Predchádzajúci nápad
'Live Event Intelligence' RUŠÍME ÚPLNE") in favor of putting all online/
live-market functionality directly inside Price Checker instead - see the
real "2.4.1 - Price Checker Live Market Monitor" entry above. Unlike 2.3.0
(reverted as **2.3.1**, a version bump forward, because that build had
already been offered as a real release), this direction was never released -
only handed over as a review package - so no install anywhere ever recorded
it, and the version number "2.4.0" was safe to reuse for the real feature
that replaced it - though that real feature's own version ultimately moved
one more step forward, to **2.4.1**, for the separate and unrelated
practical reason explained in the entry above and in
`PROJECT_STATE/CURRENT_STATE.md`'s "## Version" section. See that section
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.4.0 (pre-release direction)"
entry for the full reasoning, and re-verify that same fact (was anything
with this version/migration number ever actually installed anywhere?)
before assuming a THIRD reverted direction can reuse its number too - it
depends entirely on that, not on precedent alone.

1. **New table `event_online_sources`** (migration
   `026_live_event_intelligence.sql`) - a standalone table, not a new
   column on `events` and not a foreign key onto the general, marko-managed
   `marketplaces` lookup. `UNIQUE(event_id, source)` enforces "at most once
   per marketplace per event"; `verified`/`active` are two independent
   flags (confirmed-by-a-human vs. still-connected).
2. **Discovery, always human-confirmed.** "Find Online Event" opens a real,
   visible browser window (reusing the Visible Scanner's technique, never
   its code/state) on a best-effort search URL; marko searches himself;
   "Capture this page" reads only the current page's title+URL as one
   candidate; "Use this one" is the only action that ever saves a source as
   verified. "Refresh" is the identical flow against an already-saved URL -
   also how a manually-connected source becomes verified. "Connect
   manually" skips the window for when marko already has the URL.
3. **New compact "Live Event Intelligence" block** on EventDetail's
   Overview tab (above Inventory Intelligence) - always exactly 3 rows,
   Find Online Event / Connect manually / Refresh / Open source /
   Disconnect-Reconnect.
4. **No new networking primitive at all** - the only network access this
   feature ever performs is opening a real, visible window a human drives;
   no backend HTTP calls to any of the 3 marketplaces, ever.

19 new backend unit tests + 3 new migration-upgrade tests (existing events
untouched, CHECK constraint enforced on an upgraded database, cascade
delete verified). `cargo test --lib` (1042 passed), `npx tsc -b`, `npm run
build` all green.

## 2.3.5 - Sync/push redesign: self-healing push, real sync diff, no more UI freeze

Marko came back after 2.3.4 with one detailed message re-explaining the
whole intended sync/push design from scratch, using Pulls as the reference -
the narrow bug fixes so far hadn't matched his actual mental model. Three
fixes, all covered in depth in `PROTECTED_AREAS.md`'s new "2.3.5" entry:

1. **UI freeze fixed.** Every sync/push button froze the whole app until its
   network call finished (marko: "ked zapnem alebo kliknem na cokolvvek...
   apka zamrzne"). All 11 sheet-sync commands were plain synchronous `fn`,
   which Tauri runs on its single main/IPC thread - converted to `async fn`
   + `spawn_blocking`, same pattern already proven for Google sign-in
   (2.0.12->2.0.13). Zero changes to the underlying sync/push logic.
2. **Order/Sales sync now updates an already-linked row when the sheet
   changed it**, matching Pulls sync - previously every marked row was
   skipped unconditionally, no comparison at all. Tracks platform/date/
   currency/email/Order ID; deliberately never quantity/price (tickets
   already have exact-cent costs allocated against those - same "ask before
   touching" boundary as the 2.0.53 currency-push feature).
3. **Push Orders is now self-healing** - marko, twice: if he deletes an
   order's row from the sheet by hand and pushes again, it must notice and
   add it back, using the same code. Push Sales needed no changes at all for
   this: it never creates rows, so once Push Orders restores the row, Push
   Sales's existing "fill in blank cells" behavior already re-populates the
   sales columns on the next run - proved with a dedicated test chaining
   both functions. This also resolves the row-426 order that went
   permanently invisible after 2.3.4 (see that entry below).

9 new/updated sync-diff tests, 3 push self-healing tests, 1 cross-function
integration test. Full suite 1020/1020 passed, 0 failed; `tsc -b`/
`npm run build` clean. No frontend changes needed - the sync/push buttons'
busy-state/spinner UI already existed, it was just neutered by the backend
freeze.

## 2.3.4 - Sheets push: row placement fixed properly this time

2.3.3's fix wasn't enough - marko sent a screenshot proving it. Revenue (P)
and Profit (Q) in his real sheet had live formulas filled all the way to
row 425, even though only ~16 rows have real order data. Somewhere in this
sheet's history, `plan_sheet_structure_updates` had written formulas that
far down, and a formula is non-empty content too - so the raw `"A1:AZ"`
row count 2.3.3 anchored on was never actually small in his sheet, it
already agreed with Google's own confused auto-detection. Same bug,
different disguise.

Fixed properly: `next_append_row`/`next_append_range`
(`orders_sheet_sync.rs`) now scan for the LAST row whose **marker cell**
(TIQR ID) is non-empty - the one column only this app ever writes, and
only for a row holding a real order - and target the row right after it,
ignoring any stray formula residue further down. 5 unit tests added,
including the literal shape of marko's real sheet (16 real rows + 408
stray-formula rows still targets row 18) and a deliberate "never reuse a
gap in the middle" case. Full suite 1011/1011 passed, 0 failed; `tsc -b`/
`npm run build` clean.

Also found and documented (not fixed, not asked for): marko deleted the
row-426 order's content directly in the sheet while testing between
versions, which this app's own bookkeeping now can't see - that order
won't automatically come back. See `PROTECTED_AREAS.md`'s "2.3.2-2.3.4"
entry before doing anything about it.

Revenue/Profit formulas being missing on older rows is still believed to
be the same root cause, not independently fixed - marko needs to confirm
on his real sheet after this update. Not marked resolved yet.

## 2.3.3 - Sheets push: row placement fixed at the source

Follow-up to 2.3.2's investigation (see `PROTECTED_AREAS.md`'s "2.3.2/2.3.3"
entry for the full trail). Marko confirmed row 18 and rows 19-425 in his
real sheet are genuinely empty, and that retrying the push already once did
NOT bring back the missing Revenue/Profit formulas - which pointed at one
shared root cause rather than two separate bugs.

Fixed: `push_orders_impl` no longer hands row placement to Google's own
`append_values` table auto-detection (a bare `"A1"` anchor, which was
landing new rows at 426 instead of 18). It now computes the exact target
row itself, via a new pure, unit-tested `next_append_range` function, from
the same `"A1:AZ"` read this function already trusts for its header/
marker-column lookup, and writes with `update_values` instead. 3 new tests
added (`next_append_range_*`), all passing; full suite 1009/1009 passed, 0
failed. `tsc -b`/`npm run build` also clean (frontend untouched this
release).

Not independently touched, believed fixed as a side effect: the missing
Revenue/Profit formulas. `plan_sheet_structure_updates` already recomputes
formulas for the sheet's entire current extent on every push, so once new
rows land in the right place, the very next push should backfill formulas
correctly again. **Marko needs to click Push Orders/Push Sales once more
after this update and confirm** - not marked resolved until he does; see
`PROTECTED_AREAS.md` for exactly what to report back if it isn't.

Known, deliberate limitation: this does not move the order a past, buggy
push already stranded at row 426 in marko's real sheet - that needs a
manual fix in the sheet itself if he wants it back in the contiguous block,
since this app can't safely edit that live row unattended.

## 2.3.2 - Dashboard: all-time Total cost

Marko's request: a place on the Dashboard to see total cost across every
ticket he owns. Added a "Total cost" StatCard to the Financials tab's
existing "Current inventory (all time)" section, next to
Available/Listed/Sold (total)/Purchased (total) - zero backend change,
`data.inventory.totalCostCents`/`.currency` (a `FinanceSummary`) were
already computed and sent to the frontend every load, just never rendered
anywhere. Verified with `tsc -b`/`npm run build`/`cargo test --lib` (1006
passed, 0 failed, unaffected since no `.rs` file changed).

Also investigated (not yet fixed - see `PROTECTED_AREAS.md`'s "2.3.2"
entry) two Google Sheets sync complaints from the same message: Push
Orders/Sales landing a new row at 426 instead of at row 18 (the sheet's
real next empty row), and Revenue/Profit formulas missing on many rows.
Root-caused enough to have a credible fix shape for the first, but stopped
short of writing it - this touches marko's live, real-money Google Sheet,
and the fix's correctness depends on what's actually sitting in rows
19-425 of his real sheet, which cannot be verified from here. Asked marko
directly rather than guess. No Sheets-sync code was changed this release.

## 2.3.1 - Event Lifecycle removed (revert of 2.3.0)

Marko reviewed the 2.3.0 build below (delivered as a zip + Slovak report,
never actually published via `1-CLICK-UPDATE.bat`) and asked to remove it
entirely and go back to the previous version - no specific complaint beyond
not liking it in place ("mi tam nieje sympaticky"). `Events.tsx`/
`EventDetail.tsx` were reverted edit-for-edit to their exact pre-2.3.0
content (verified with `tsc -b`/`npm run build`/`cargo test --lib`, all
clean). The version was NOT rolled back to 2.2.12, even though the code
was - marko himself caught this right after asking for the revert: reusing
an old version number breaks the auto-updater for anyone already offered a
newer one ("ked dam stary tak to nefunguje potom dobre"). So this reverted
build ships as **2.3.1** instead (all 9 locations) - a version bump
forward that happens to contain strictly less than the 2.3.0 it follows.
**Lesson for future sessions:** a revert-to-previous-behavior task still
needs a version bump forward, never a rollback to a number already used
before - the updater compares versions directly. The entry right below is
kept as real history (this file is append-only), not deleted - see
`PROJECT_STATE/CURRENT_STATE.md`'s matching note for what to do if a similar
feature is requested again.

## 2.3.0 - Event Lifecycle / Event Operations (reverted - see entry above)

Marko's next task after 2.2.11/2.2.12: one consistent, derived "what stage
is this event at" lifecycle phase - no new manually-set status, no
migration, no backend change at all (`cargo test --lib` byte-for-byte
unchanged from 2.2.12). See `REDESIGN-2.3.0-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.3.0" entry.

- **6 phases** - UPCOMING -> INVENTORY -> LISTED -> SELLING -> EVENT DAY ->
  COMPLETED (`computeEventLifecyclePhase`, `Events.tsx`) - a pure function
  of the already-returned `EventWithStats`, zero extra IPC calls. His
  proposed POST EVENT is folded into COMPLETED (his own literal COMPLETED
  rule leaves no gap to place it in); CANCELLED stays inside COMPLETED too
  (it already has its own Status badge). Both judgment calls explained in
  `PROTECTED_AREAS.md`.
- **Events overview**: lifecycle phase shown as a small pill stacked under
  the existing Status badge (no new column/colgroup change), plus a new
  "Lifecycle phase" filter dropdown, ANDed with the existing Upcoming/
  Completed tab.
- **Event Workspace (Overview tab)**: new `EventLifecycleBlock` at the top -
  current phase, a simple progress strip, an operational summary line
  (tickets/listed/sold/pending fulfillment), and a "Next Actions" list -
  sourced entirely from already-existing `list_sale_groups` (per-event
  `isSaleGroupDone`) and `get_attention_center` (global, filtered to this
  event) data, no new business logic.
- **Tests**: 25/25 on a disposable esbuild+Node script exercising marko's
  full scenario list (upcoming with/without inventory, listings, sales,
  event day, date passed, completed/cancelled, phase precedence,
  filter-by-phase, pending fulfillment, Next Actions aggregation) against
  the real exported functions. `cargo test --lib`: 1006 passed, 0 failed, 3
  ignored - unchanged, no `.rs` file touched. `tsc -b`/`npm run build`: 0
  errors.

## 2.2.12 - Fulfillment Center

Marko's ČASŤ C, shipped as its own release right after 2.2.11 (same
message, explicitly split into two releases). See
`REDESIGN-2.2.12-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.12" entry. Zero backend/migration
changes - frontend only.

- **New page: Fulfillment Center** (`src/pages/FulfillmentCenter.tsx`, new
  `/fulfillment` sidebar entry right after Sales) - one place to see every
  sold ticket not yet fully paid, delivered, and completed. Fetches the
  same `SaleGroup[]` Sales.tsx already fetches and reuses its exact
  `isSaleGroupDone` rule (now exported) - no parallel status system, no new
  backend command.
- **4 clickable tiles double as KPIs and category filters**: Pending Sales
  (all), Awaiting Payment, Awaiting Delivery, Ready to Complete - the last
  one a new, pure display derivation (paid + delivered in full; the only
  way such a group is still Pending is a partial refund).
- **Table**: Event / Ticket+Seats / Sale price / Payment status / Delivery
  status (new group-level badge, existing tone colors) / Overall status /
  Action - row click or the Action button both open the existing Sale
  Detail page (`/sales/:id`), no new navigation mechanism.
- Verified with a disposable, esbuild-bundled Node script (built and run
  once, then deleted) asserting all of marko's listed test scenarios
  against the real exported functions - 21/21 passed - since this codebase
  has no frontend test framework.

## 2.2.11 - Attention Center UX rework + Dashboard cleanup

Marko's own next request, split into two explicit parts, both frontend-only
(zero backend/migration changes). See `REDESIGN-2.2.11-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.11" entry for the judgment
calls behind each.

- **Attention Center reworked from one mixed feed into 5 named, always-
  visible boxes** (`Dashboard.tsx`): NO LISTING PRICE YET / NO ACTIVE
  LISTING / NOT DELIVERED YET / EVENT COMING SOON / MARKET ATTENTION -
  grouped by the item's existing `category` field instead of `priority`.
  Clicking a box reveals only that category's own rows (reusing
  `AttentionCenterRow` unchanged); the old mixed feed is gone as default
  content. A box with 0 items is disabled, not hidden. Zero backend
  changes - `attention_center.rs` already satisfied every MARKET ATTENTION
  constraint (Price-Checker-gated, no automatic pricing, section/row/tier
  never a pricing factor), confirmed by reading its own doc comment and
  tests rather than assumed. `AttentionSection`/the alert bell (older,
  separate feature) are untouched.
- **Dashboard Overview: unbounded "Sales by platform" list capped** with
  its own `max-h-72 overflow-y-auto`, so a long platform list scrolls
  internally instead of growing the whole page - paired with a small,
  one-step trim of two existing spacing values on the same tab (not a
  redesign). `Layout.tsx`'s scroll container was checked and needed no
  change.

## 2.2.10 - Eight follow-ups from marko's 2.2.9 review

Marko reviewed 2.2.9 and sent two rapid-fire messages (7 screenshots) with
eight mostly-unrelated requests. See `REDESIGN-2.2.10-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.10" entry for the judgment
calls behind each. No migration this release.

- **Seats format: dropped the "Sec"/"Row"/"Seat" labels 2.2.9 had just
  added**, back to bare " · "-joined values (`formatSeatLocation`/
  `formatSeatsSummary`, `lib/format.ts`) - a real section value ("Sec 408",
  "Category D, Standing") sometimes already read as a full label, so the
  prefix produced "Sec Sec 408"-style duplication. Reaches every "Seats"
  column across the app via the same two shared helpers.
- **Orders tabs reworked: "Active"/"Paid" -> "Active"/"Completed"**, with a
  real bucketing change, not just a relabel - an order is now Completed
  once its event's date has passed (or its status is completed/cancelled)
  OR the order itself is fully sold+delivered+paid, whichever comes first.
  The New Order event picker now excludes those same "done" events too
  (previously unfiltered).
- **Attention Center "mixed" ordering fixed** - the real cause was the
  sort's own tie-break (grouping by category name before order), not the
  2.2.9 grouping-by-order logic itself. Also now excludes done events from
  3 of its 5 categories (missing listing price/no active listing/outside
  market price) - `sold_undelivered` and `event_soon` are deliberately
  exempt.
- **Sales Pending/Completed now requires sold+delivered+paid together**
  (or fully refunded) - a sale missing only its delivery status no longer
  incorrectly showed as Completed.
- **Two confirmed Google Sheets push bugs fixed** (Orders and Pulls push):
  local "already synced" bookkeeping was being written before the actual
  network write was even attempted, so a failed push still silently looked
  successful afterward. Both now record success only after the sheet write
  is confirmed.
- **Google's `invalid_grant` sign-in error now shows a short "sign in
  again" message** instead of a long raw JSON dump - best-effort fix for a
  reported long error after Google sign-in; not independently reproducible
  in this environment.
- **Native right-click context menu disabled everywhere** (no config flag
  exists for this in Tauri/WRY - the standard JS-side `preventDefault` fix).

Verified: `cargo test --lib` (1006 passed, +7 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.9 - Six follow-ups from marko's 2.2.8 review

Marko reviewed the just-shipped 2.2.8 Attention Center and sent six mostly-
unrelated small requests in one message. See `REDESIGN-2.2.9-REPORT.md`
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.9" entry for the
judgment calls behind each.

- **Seatriks retired from Price Checker only** (`marketplaces.active = 0`,
  `migrations/025_deactivate_seatriks_price_checker.sql`) - same mechanism
  already used for StubHub. Stays fully available in Listings' "Add
  listing" picker.
- **Settings -> Integrations' Anthropic API key card relabeled** from
  "AI-assisted price reading" to the general "AI features" - same key,
  same storage, just a forward-looking name since it's meant to power more
  than one AI feature over time.
- **No live "balance" number was built** - Anthropic's API has no endpoint
  that returns a remaining credit balance for any key type (confirmed
  against Anthropic's own docs). A "Check usage & balance" link to the
  Anthropic Console was added on the same card instead of fabricating one.
- **Finance -> Overview gained "New entry"/"New account" buttons**, opening
  the exact same modals already used on the Transactions/Accounts tabs.
- **The per-event "Attention" list was removed from Event Workspace**
  (Inventory Intelligence's own 2.2.6 block) - fully superseded by the
  Dashboard's global Attention Center. The backend it was built on is
  untouched, since the Attention Center itself still depends on it.
- **Attention Center (2.2.8) reworked to group by order.** The four
  ticket-level categories now emit one row per order instead of one row
  per ticket - marko's own example was a 49-ticket order shown as 49 rows.
  Clicking a grouped row opens that order's own page, which already lists
  every affected ticket with its own status/price/delivery indicators.
  `event_soon` is unchanged (still one row per event).
- **Seats display reformatted: the "/" is gone.** `formatSeatsSummary`
  (Orders/Tickets/Inventory/Sales/Pulls' "Seats" columns) now shows
  clearly labeled, separated Section/Row/Seat text (e.g.
  "Sec 402 · Row 56 · Seat 27") instead of a bare slash-joined pair, and
  eight duplicate ad-hoc "/" joins across Sales.tsx and EventDetail.tsx
  were consolidated into the same shared formatter.

Verified: `cargo test --lib` (999 passed, +4 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.8 - Dashboard global "Attention Center"

Focused task on top of 2.2.6/2.2.7: a new compact Dashboard block (Activity
tab) listing individual things across EVERY event that currently need a
look, grouped by priority (Critical/Attention/Info) and sorted by priority
then soonest event. See `REDESIGN-2.2.8-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.8" entry for the
judgment calls behind it.

- **New backend command**: `get_attention_center`
  (`commands/attention_center.rs`, new file) - no migration, no new
  dependency.
- **Four of five categories reuse 2.2.6's exact per-event Inventory
  Intelligence "Attention" rules** (event within 2 days with unsold
  tickets, unsold ticket with no listing price, unsold ticket with no
  active listing, unsold ticket priced 20%+ off market average - only with
  real Price Checker data), flattened into individual clickable rows.
- **New fifth category**: sold ticket whose `delivery_status` isn't
  literally `"Delivered"` yet - reuses the exact convention 2.0.66's
  "Completed" indicator already established; a refund excludes itself
  automatically (ticket reverts to `available`).
- **Navigation**: reuses `Tickets.tsx`'s existing `?code=` deep link for
  ticket-level rows, and `/events/:id` for the one event-level category
  (`event_soon`) - no new route/navigation mechanism.
- **Display**: reuses the Activity tab's existing `ShowMoreToggle`/
  `RECENT_LIST_PREVIEW_COUNT` pattern per priority group - the backend
  never truncates.
- Deliberately does NOT touch the existing Dashboard alert bell/"Attention"
  cards (pulls/pending sales/missing listing price by order/upcoming
  events) - a separate, additional block, not a replacement.
- No automatic pricing/repricing anywhere; `tier`/`section`/`row` are never
  used as a pricing factor.
- **+10 new Rust unit tests** (event-soon in/out of window, unsold ticket
  without active listing, unsold ticket without listing price, market
  alert only with real Price Checker data, sold-undelivered fires/excludes
  delivered/excludes refunded, sold-undelivered priority window, sold-out
  event still flags undelivered tickets, same ticket under 2 categories
  never twice under 1, priority+date sort order). Full suite: 995 passed /
  0 failed / 3 ignored.

## 2.2.7 - Ticket metadata: Tier / Level

Focused task on top of 2.2.6: every ticket can now optionally carry a
tier/level value (e.g. "VIP", "Lower Bowl", "Level 200"), kept strictly
separate from `ticket_type` (a delivery method, not a price tier - the
same mix-up flagged twice before, now resolved for good). See
`REDESIGN-2.2.7-REPORT.md` for the full report (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.7" entry for the judgment calls
behind it.

- **New column**: `tickets.tier TEXT`, nullable (`migrations/
  024_ticket_tier.sql`, forward-only). Every existing ticket got NULL - no
  guessed/inferred values, per marko's own explicit instruction.
- **Entry points**: set at order creation (`OrderFormModal`, copied onto
  every generated ticket, same as section/row) and editable per-ticket
  afterward (`TicketEditModal`) - both small, plain text fields, no
  redesign.
- **CSV**: import accepts `tier` (or `level` as a synonym); fully backward
  compatible with CSVs that predate this column. Export (tickets, sales,
  and the downloadable order-import template) all include `tier`, right
  after `row`.
- **Inventory Intelligence** (2.2.6) gained a real "By tier" breakdown -
  blank/null shows as "Unknown"; clicking a tier group filters the Tickets
  table exactly like the section/marketplace breakdowns already do.
- **Deliberately not wired up this round** (prepare-the-data, not
  wire-it-in-yet): Market Analysis / Repricing's `YourTicketGroup.tier`
  still always reports `None`; no bulk-tier-edit action; no tier column
  added to any list/table view; Google Sheets Order sync not wired to
  `tier`. Zero changes to refund/resell, `batch_id`, money/cents logic,
  Orders/Sales/Finance core logic, Listings, or Price Checker scraping.
- Tests: +13 new Rust unit tests (migration upgrade/fresh-db, ticket
  create/update with tier, CSV import old/new format + the `level`
  synonym, CSV export tier presence for tickets/sales/template, Inventory
  Intelligence tier grouping). Full suite: 985 passed / 0 failed / 3
  ignored. `tsc -b` and `npm run build` both clean.

## 2.2.6 - Inventory Intelligence for Event Workspace

Focused task on top of 2.2.5: a compact "Inventory Intelligence" block
added to the Event Workspace's Overview tab, above the existing Orders/
Tickets tables. See `REDESIGN-2.2.6-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.6" entry for the
judgment calls behind it.

- **KPIs**: Total tickets, Total invested, Current listed value (active
  `ticket_listings` only), Potential profit (legacy `listing_price_cents`
  field, matching Sales' existing card), Sell-through %, Average ticket
  cost - all reusing existing money definitions, no new duplicate
  computations.
- **Aging**: 0-7 / 8-30 / 31-60 / 61+ days since purchase, unsold tickets
  only.
- **Attention**: event within 2 days with unsold stock, unsold ticket with
  no listing price, unsold ticket with no active listing, unsold ticket
  priced 20%+ off the market average (reuses Price Checker's own summary;
  shown as "not available yet" rather than a fake zero when this event has
  no Price Checker data).
- **Breakdown** by section and by marketplace. No "by tier" breakdown -
  `tickets` has no tier/level column anywhere in this schema; the UI says
  so in plain text instead of inventing fallback data, per marko's own
  explicit instruction.
- **Every row is clickable** - filters Overview's own Tickets table to the
  relevant tickets (or switches to the Listings tab, for "Current listed
  value"). No changes to Tickets.tsx, Orders.tsx, or any core Orders/
  Tickets/Sales/refund/resell logic; Finance page untouched.
- New backend: `commands/inventory_intelligence.rs` (1 new command,
  `get_inventory_intelligence`), no migration, no new dependency.
- Tests: +13 new Rust unit tests (KPI scope/formula parity with existing
  screens, aging bucket boundaries, attention-item independence and
  availability, currency-mixed handling, section/marketplace grouping).
  Full suite: 972 passed / 0 failed / 3 ignored. `tsc -b` and
  `npm run build` both clean.

## 2.2.5 - Event Workspace down to 3 tabs; Listings gets filters, search and bulk actions

Fourth pass on the Event Workspace, plus a Price Checker lookup addition.
Final tab order: `Overview | Listings | Sales`.

- **Sales absorbed Finance** - "sales a finance daj dokopy" (unambiguous
  this round). Finance's entries table now renders below Sales' own table
  (and below the Market section 2.2.4 already put there). See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.5" entry for the judgment call
  behind Sales (not Finance) surviving as the name.
- **Listings: filters, search, multi-select, bulk actions.** Status filter
  (All/Active/Sold/Removed), marketplace filter, search box, always-visible
  row checkboxes with select-all/deselect-all (scoped to the currently
  filtered/searched rows), and a bulk action bar (shown only while
  something is selected) for Edit status / Edit price / Delete - each
  backed by a new **all-or-nothing** transactional Rust command
  (`bulk_update_ticket_listings_status`, `bulk_update_ticket_listings_price`,
  `bulk_delete_ticket_listings`, all in `ticket_listings.rs`). Bulk price
  edit is refused, on both the frontend and the backend, when the selection
  spans more than one currency.
- **"Add listing" ticket picker rebuilt** as an order-browse flow (search
  this event's own orders, open one, pick tickets from it) mirroring
  Sales.tsx's own New Sale flow, replacing the old flat "every ticket in
  the event in one dropdown" picker. Several tickets can be picked at once,
  creating one listing per ticket on the chosen marketplace (per-ticket
  price, with a quick-fill/apply-to-all helper); Listing ID/URL are offered
  only when exactly one ticket is selected. This create flow is NOT
  all-or-nothing (unlike the 3 bulk actions above) - a partial failure
  keeps whatever succeeded and reports the rest for retry.
- **Marketplaces: added Seatriks** - new pure-data migration
  `023_add_seatriks_marketplace.sql`, no schema change.
- Existing tickets/inventory/sales/refund logic untouched; no automatic
  listing creation, marketplace API, or repricing added.
- Tests: +11 new Rust unit tests for the 3 bulk commands (selection
  scoping, invalid input, mixed currency, dedup, all-or-nothing transaction
  safety), plus 1 existing Price Checker test updated for the new 4th
  active marketplace.

## 2.2.4 - Event Workspace down to 4 tabs; Listings is now a real multi-marketplace system

Third pass on the Event Workspace. Final tab order: `Overview | Listings |
Sales | Finance`.

- **Overview absorbed Inventory** - the Orders/Tickets tables now render
  below Overview's own stat cards instead of having their own tab.
- **Sales absorbed Market** - "Market vs. mine" and "Potential Profit" (the
  former Market tab's content) now render below the Sales table. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry for the judgment call
  behind Market landing in Sales rather than Finance.
- **Finance is unchanged**, still its own tab.
- **Listings rebuilt into a real system.** New `ticket_listings` table
  (`migrations/022_ticket_listings.sql`) - one ticket can now have several
  listings at once, one per marketplace (reuses the existing `marketplaces`
  lookup table), each with its own price/currency/status/listing id/URL/
  last-updated timestamp. Full add/edit/delete UI in the tab; summary cards
  count active listings only, the table shows every listing regardless of
  status. Manual entry only - no marketplace API, no automatic listing
  creation, no repricing. Never touches `tickets.status`/
  `tickets.listingPriceCents`.
- New backend: `commands/ticket_listings.rs` (4 commands: list-for-event,
  create, update, delete) + `commands::price_checker::
  delete_marketplace_impl`'s existing guard extended to also count
  `ticket_listings` (so deleting a marketplace with real listings against
  it is refused, same as it already was for saved links/price-check
  history).

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry before extending any
of these tabs or the new table further.

`cargo test --lib` (948 passed, up from 934 - 14 new tests: 13 for
`ticket_listings`, 1 for the `delete_marketplace_impl` guard extension),
`tsc -b`/`vite build` clean. One new migration (022); no changes to
existing tickets/orders/sales/refund logic. Full detail in
`REDESIGN-2.2.4-REPORT.md`.

## 2.2.3 - Event Workspace: Listings tab, Tasks removed, tables full-width

Second pass on the Event Workspace, all frontend-only. Final tab order:
`Overview | Inventory | Listings | Sales | Market | Finance`.

- **Tasks tab removed entirely** - marko decided against it before it had
  a spec; it was only ever an empty placeholder, so there was nothing to
  migrate.
- **New Listings tab** - a read-only view of this event's tickets already
  filtered to `status === "listed"`: ticket, listing price, currency,
  status, plus an Active listings/Listed value/Lowest/Highest summary.
  Deliberately does NOT show marketplace, listing URL, or last checked -
  none of the three exist anywhere in the `tickets` schema (checked all 21
  migrations) or in Price Checker's own listing data, and marko explicitly
  asked not to invent data that isn't real. The tab says so plainly.
  Reuses the same `tickets` array Inventory already loads - no new API.
- **All 4 Event Workspace tables (Orders, Tickets, Sales, Finance) now
  fill the window width** - removed the `max-w-[1400px]` cap that was
  stopping them short of the right edge, the same fix `Layout.tsx` itself
  got in 2.0.31.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.3" entry before extending
any of these tabs further.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.3-REPORT.md`.

## 2.2.2 - Event Workspace, plus 3 small fixes

`EventDetail.tsx` is now a tabbed "Event Workspace"
(`Overview | Inventory | Sales | Market | Finance | Tasks`, via the same
`TabSwitcher` Tickets.tsx/Events.tsx already use):

- **Overview** - exactly marko's own list (tickets, sold, available,
  total cost, revenue, profit, margin, ROI), no backend change.
- **Inventory** - the existing Orders + Tickets tables, unchanged, just
  relocated under their own tab.
- **Sales** - `list_sale_groups({ eventId })` (Sales.tsx's own Event
  filter, reused), compact table, "Open in Sales" for more.
- **Market** - `get_price_checker_summary(eventId)` (PriceChecker.tsx's
  summary command, reused) plus the existing "Potential Profit" block,
  now together in one tab.
- **Finance** - `list_finance_entries_for_order` (2.2.1) called once per
  this event's own orders, merged client-side - no new backend command.
- **Tasks** - honest placeholder (`EmptyState`), no spec given yet.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.2" entry before extending
any of these tabs.

Plus three unrelated small fixes: Settings -> Lookups' 3 category lists
no longer cap their scroll area at a fixed 224px (`max-h-[60vh]` now);
Event Detail's last table (Tickets) was missing the `mb-8` its Orders
neighbor had, so the Potential Profit box after it read as crammed
against it; Price Checker's event picker now only lists
`status === "upcoming"` events, so a completed/cancelled event quietly
stops showing up there.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.2-REPORT.md`.

## 2.2.1 - Finance Accounts redesign, Settings Lookups redesign, Price Checker jump links, Finance-Orders linking

Four independent, marko-requested pieces in one release:

- **Finance Accounts** (`src/pages/finance/Accounts.tsx`): the old
  `sm:grid-cols-2 lg:grid-cols-3` grid of large `AccountCard`s replaced
  with one compact divide-y list (`AccountRow`) - same dense-row visual
  language as PlatformList/EventCategoryList. Balance stays the most
  prominent number per row; opening balance moved to a hover tooltip.
- **Settings -> Lookups** (`Settings.tsx`): was one long always-expanded
  Card (Platforms/Event categories/Finance categories); now exactly 3
  clickable summary rows (same row/chevron style as Settings Home's own
  section list), each opening its list(s) in a Modal. The add/delete
  functionality itself is unchanged - only the container is new.
- **Price Checker jump links**: "Check prices" button added to
  `OrderDetail.tsx` and `SaleDetail.tsx` (hidden on a sale group spanning
  mixed events), reusing the exact `navigate("/price-checker", { state: {
  presetEventId } })` pattern `EventDetail.tsx` already used since 2.0.81;
  `PriceChecker.tsx` needed no changes.
- **Finance <-> Orders linking**: `finance_entries.order_id` (new
  `migrations/021_finance_entry_order_link.sql`, `ON DELETE SET NULL`) -
  a deliberate, marko-confirmed (via question) reversal of ONE part of
  `015_finance.sql`'s original "fully independent ledger" design. New
  "Record in Finance" button/modal on `OrderDetail.tsx` pre-fills a new
  expense entry from the order's own amount/currency/date, with
  amount/currency locked read-only so the two numbers can never drift
  apart. New `list_finance_entries_for_order` command. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.1" entry before touching
  `finance_entries.rs` again - in particular the "must round-trip
  `orderId` unchanged on edit" trap, already fixed proactively in
  `Transactions.tsx` and `Overview.tsx`.

6 new Rust tests. 934 passed / 0 failed / 3 ignored (up from 928), clippy
clean, `tsc -b`/`vite build` clean. Full detail, including the
AskUserQuestion decision on the Finance-Orders link design, in
`REDESIGN-2.2.1-REPORT.md`.

## 2.2.0 - Price Checker Market Analysis

New `commands/price_checker_analysis.rs` (2 commands) derives tier/section
price breakdowns, comparable-ticket ranking, and Your Tickets price
recommendations from a Visible Scanner session's already-accumulated
listings - never touches the scanner's own session/lifecycle code.
`migrations/019_price_checker_market_analysis.sql` adds `price_check_tiers`
so saved checks remember a per-tier breakdown going forward. 40 new Rust
unit/integration tests (incl. 2 added during this release's own adversarial
review pass, after finding tier/section grouping was case-sensitive while
comparable-matching already wasn't - see `PROJECT_STATE/PROTECTED_AREAS.md`'s
"2.2.0" entry). Full detail, including every flagged design decision, in
`PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md`.

## 2.2.0 - StubHub fully removed, including history

`migrations/020_remove_stubhub.sql` deletes the StubHub marketplace row and
every `price_checks`/`price_check_tiers`/`event_marketplace_links` row that
ever referenced it - marko's own explicit, confirmed decision to go further
than 2.1.6's "keep history, stop offering it for new checks." Irreversible
by design.

## 2.1.9 - PROJECT_STATE protocol adopted

Set up `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/KNOWN_BUGS.md`,
and `PROJECT_STATE/PROTECTED_AREAS.md` (moved verbatim from the old root
`PROTECTED-AREAS-NOTES.md`, which is now a pointer stub) per marko's
development protocol. No code changes. `KNOWN_BUGS.md` starts empty by
design - see its own header for why.

## 2.1.9 - Price Checker Visible Scanner

Replaced the hidden auto-check WebView with a visible one the user scans
himself. Full detail in `PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md` and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.1.9" entry.

## 2.0.80 - Google Sheets Summary block: Paid-gated Revenue/Profit + refund staleness fix

- `plan_orders_summary_updates` (`orders_sheet_sync.rs`): "Total
  Revenue"/"Total Profit" now use the same Paid-gated `SUMPRODUCT` as
  "Total Paid", instead of summing every sold row regardless of payment
  status. Confirmed with marko via question before implementing.
- New `order_fully_refunded` check + clearing branch in
  `apply_sales_push_internal`: once every ticket on an order has been
  refunded, "Push sales"/"Fix sync" now blank that row's 7 Sales-sync
  columns instead of leaving stale pre-refund data forever (previously not
  even "Fix sync" could correct it - `uniform_sale_for_order` returns
  `None` for a refunded order, so nothing detected the drift).
- 9 new Rust tests. 747 passed / 0 failed / 3 ignored.

## 2.0.79 - Dashboard cleanup + CSV export staleness fixes

- Removed the Dashboard Overview tab's Quick Actions button row
  (New Event/Order/Sale, Import/Export CSV) - redundant with each page's
  own button and Settings -> Data.
- Orders/Tickets/Inventory/Sales CSV exports had drifted behind the data
  model over many versions; added the missing columns (event category;
  resale/delivery status; order code, seat location, margin, ROI,
  resale/delivery status, refund details).
- Dashboard Activity's "Unpaid payments" tile replaced with "Pulls near
  deadline" (pulls not yet transferred, event date approaching/past) -
  reuses Pulls.tsx's own existing warning window rather than a new rule.
  `unpaid_orders_count` itself is untouched (still used by notifications).
- 738 passed / 0 failed / 3 ignored.

## 2.0.78 - Pushover -> ntfy

- Swapped the Pushover notification channel for ntfy (no built-in app
  token needed).

## 2.0.77 - Notification simplification

- Removed the email notification channel entirely (SMTP config, `lettre`
  dependency) at marko's request.
- Pushover simplified to user-key-only; app token is now built into the
  binary via a GitHub Actions secret, same pattern as other embedded keys.
- 732 passed / 0 failed / 3 ignored (29 in the notifications module).

## 2.0.76 - Outbound notifications: desktop, email, Pushover

- New `commands/notifications.rs`: desktop (tauri-plugin-notification),
  email (SMTP via `lettre`), Pushover channels. Settings -> Notifications
  with a "Send test" button per channel.
- Background check every 30 min (+ once on launch) against the same 4
  Dashboard "Attention" categories the alert bell (2.0.75) already shows;
  max once per category per calendar day; upcoming-events only pushes
  within a 3-day window (vs. the bell's 14-day display window).
- Secrets stored plain-text in `app_settings`, same existing trust
  boundary as the rest of the app; never echoed back to the UI.
- Only fires while the app process is running - no tray/background
  service (documented limitation, not a bug).
- 729 passed / 0 failed / 3 ignored (26 new).

## 2.0.75 - Dashboard alert bell

- New `AlertBell` on the Dashboard (top-right, next to the tab switcher):
  badge counts how many of the same 4 "Attention" categories are non-zero;
  amber, red only when the soonest upcoming event is due today/overdue.
  Reuses the exact same numbers `DashboardAlerts` already computes - no
  new backend logic. Frontend-only change.
- 703 passed / 0 failed / 3 ignored (unchanged - no Rust touched).
