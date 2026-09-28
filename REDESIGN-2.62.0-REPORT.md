# TIQR Manager 2.62.0 — poznámky sa dajú používať

Toto je tá verzia z náhľadu, ktorý si odklepol, plus tie dve veci, čo si dopísal
neskôr: **posuvník šírky** a **dva bloky vedľa seba**. A to tretie — *„to
posuvanie nielen na riadky ale na vsetky zlozky tam"* — ťahanie funguje na
riadkoch, na podkartách aj na poznámkach.

**Žiadna migrácia.** Najskôr som sa pozrel do schémy a nič nové nebolo treba.
Jediné pole, ktoré to vyzeralo chcieť — popis obrázka — už existuje od 2.58.0
(`note_images.caption`), len sa doň nikdy nič nepísalo. Teraz sa píše.

---

## Priradenia

### Štítok je odkaz

Klikneš na štítok pri poznámke a otvorí sa to, k čomu je podkarta priradená.

| Typ | Kam to ide |
|---|---|
| Objednávka | otvorí tú objednávku |
| Event | otvorí ten event |
| Predaj | otvorí ten predaj |
| **Lístok** | otvorí **objednávku**, v ktorej ten lístok je |
| **Pull** | otvorí **zoznam pullov** |
| **Financie** | otvoria **Financie** |

### To, čo chcem, aby si vedel

**Posledné tri riadky tabuľky nemajú v TIQR vlastnú stránku.** Lístok, pull ani
jednotlivý finančný záznam sa nedajú „rozkliknúť" — také stránky v appke nie sú.

Nerobil som ich. Zadanie hovorilo *„Do NOT create new modules or duplicate
existing detail pages"*, tak som odkaz nasmeroval na najbližšie, čo existuje.
Funguje to a dostaneš sa tam, kam chceš — len to nie je detail toho jedného
lístka.

**Keď chceš, aby lístok, pull alebo financia mali vlastnú stránku, napíš a
spravím to ako samostatnú vec.** Je to jedno miesto v kóde (`link_href`), takže
to nie je veľká robota — ale je to nová stránka, a tú som ti teraz nemal robiť.

---

## Kopírovanie

### Duplikovať podkartu, duplikovať poznámku

V menu **⋯** pri podkarte aj pri poznámke.

**Vždy sa spýta jednu vec:** *len obsah*, alebo *aj s priradeniami*?

Nedal som tomu predvolenú odpoveď naschvál. Keby kópia ticho ťahala priradenia
so sebou, všimneš si to až keď niečo pošleš dvakrát. Keby ich ticho zahodila,
všimneš si to až keď ich budeš hľadať. Ani jedno nie je dobré ticho.

Kópia sa volá **„… copy"**, ďalšia **„… copy 2"** — nikdy sa netrafí do názvu,
ktorý už máš. Poznámka bez názvu sa skopíruje ako „Bez názvu copy".

Kópia sa postaví **hneď pod originál**, nie na koniec zoznamu.

### Obrázky v kópii

Kópia **celej poznámky si berie aj svoje obrázky** — inak by v nej každá fotka
písala „Obrázok sa nenašiel". Obrázky totiž patria poznámke, nie riadku.

Kópia **podkarty** obrázky nekopíruje a ani nemusí — ostáva v tej istej
poznámke, ktorá ich už má. To je aj lacnejšie: obrázky sú v databáze a sync
nahráva celý súbor, takže zbytočná kópia by ťa stála dáta pri každom syncu.

---

## Presúvanie

**Ťahaním myšou** sa presúvajú:

- **riadky** — chytíš ich za úchyt `⠿` naľavo (naschvál nie za celý riadok —
  inak by sa v ňom nedal označiť text)
- **podkarty** — chytíš kartu a pustíš ju na inú
- **poznámky** v ľavom zozname

Kam to pustíš, tam to skončí — pri ťahaní hore aj dole rovnako, takže dva
susedné si vymenia miesto. Šípky ↑ ↓ ‹ › ostali, ak sa ti ťahať nechce.

---

## Písanie

### Odrážky a číslovanie

Dve nové tlačidlá: **• Odrážka** a **1. Číslovanie**.

Čísla sa **prepočítavajú samé**. Keď vložíš riadok do stredu, zvyšok sa
prečísluje, a keď niečo zmažeš, tiež. Číslo sa nikam neukladá — počíta sa z
toho, koľko číslovaných riadkov je nad ním. Takto sa nikdy nemôže stať, že dva
riadky si myslia, že sú tretie.

### Dva bloky vedľa seba

> *„ze si viem pridat 2 riadky vedla seba"*

Tlačidlo **⬓ Na polovicu**. Dva bloky na polovicu sa postavia vedľa seba —
presne ten druhý obrázok, čo si poslal.

Funguje to **na všetkom**: na texte, nadpise, zaškrtávacom riadku, odrážke,
**obrázku aj čiare**. To bola tá tvoja druhá veta — *„na vsetky zlozky tam"*.

### Posuvník šírky

> *„ze si viem zmenit velkost toho kde sa pise posuvnikom"*

Vpravo hore v paneli. Od 520 do 1180 px.

Pamätá si to **tento počítač** — nie poznámka. Takže na Macu môžeš mať úzko a
na Windows široko a nebijú sa. Do databázy to nejde a nikam sa to neposiela.

### Ctrl+F

Nájde v celej poznámke — v názve, vo všetkých riadkoch aj v popisoch obrázkov.
Ukáže `3 / 11` a šípkami ‹ › skáčeš medzi nálezmi.

**Kurzor ti sadne rovno na nájdené slovo**, nie je to len farebné zvýraznenie.
Môžeš hneď písať.

---

## Obrázky

- **klikneš naň a otvorí sa na celú veľkosť** (Esc alebo klik vedľa ho zavrie)
- **má vlastný popis** — riadok pod obrázkom, píšeš doň priamo
- zmazať sa dá aj z toho veľkého okna, vždy sa najprv spýta

---

## Zoznamy a menu

### Koľko je hotové

Keď má podkarta zaškrtávacie riadky, hore vidíš **„4 / 7 hotových"** a prúžok.

### Menu ⋯ pri podkarte

Premenovať · Duplikovať · Posunúť doľava/doprava · Zmazať podkartu

### Menu ⋯ pri poznámke

Duplikovať · Pripnúť/Odopnúť · Archivovať · Zmazať poznámku

### Zmazať poznámku sa presunulo

Bolo to tlačidlo v paneli nástrojov, hneď vedľa formátovania. Teraz je v menu
**⋯** pri poznámke v zozname — teda tam, kde sa poznámka spravuje. Pýta sa
presne tak isto ako predtým, nič sa neuľahčilo.

### Šablóny pre novú poznámku

Keď dáš **+**, spýta sa na šablónu: **Prázdna, Event, Objednávka, Pull,
Predaj**.

**Šablóna len pripraví podkarty. Nič do nich nenapíše.** „Event" ti spraví
podkarty *Event · Lístky · Nákup · Prevod · Predaj · Poznámky* — prázdne.
Prázdna podkarta „Platba" je miesto, kam si napíšeš, nie tvrdenie, že je
zaplatené.

---

## Čo som overil a čo nie

**Overené — naozaj spustené, nie prečítané:**

- **duplikovanie celej poznámky** na naozajstnej schéme: kópia má vlastné
  obrázky, jej riadky ukazujú na ne, **originál ostal nedotknutý**, a keď kópiu
  zmažeš, obrázky originálu tam sú
- **„len obsah" naozaj nekopíruje ani jedno priradenie**, „aj s priradeniami"
  ich skopíruje všetky
- **názvy kópií sa nikdy nezrazia** — skúšané na prázdnych názvoch aj na
  poznámke, ktorá už kópiu má
- **duplikovanie podkarty**: kópia si sadne hneď za originál a poradie ostane
  `0..n-1` bez dier
- **ťahanie**: všetky kombinácie „odkiaľ-kam" pri 2 až 8 položkách — nič sa
  nestratí a potiahnutá vec skončí presne tam, kam si ju pustil
- **číslovanie**: vkladanie, mazanie aj prerušenie obyčajným riadkom
- **Ctrl+F**: veľké/malé písmená, viac nálezov v jednom riadku, prekrývajúce sa
  slová (nezacyklí sa), prázdne hľadanie
- **38 migrácií** na čistej databáze — 2.62.0 nepridáva žiadnu
- **59 SQL príkazov** modulu poznámok proti skutočnej schéme, 0 zlých
- **241 príkazov** Rust ↔ appka 1:1, vrátane troch nových
- **poradie zlučovania** pri syncu: žiadna tabuľka sa nezlučuje skôr než tá, na
  ktorú ukazuje
- každý nový riadok, ktorý duplikovanie zapíše, **dostane svoje `uid`** — čiže
  kópia sa naozaj prenesie na druhý počítač

**Neoverené:** appku nezostavím — na tomto Macu nie je ani Node, ani Rust. To
spraví tvoj build. A to, ako to celé sadne pod prstami, vieš povedať len ty.

---

## Čo som nespravil

- **nové stránky pre lístok / pull / financie** — píšem o tom vyššie, čakám na
  tvoje slovo
- **sync** — do toho som v tejto verzii nesiahol. Stále čakám na ten screenshot
  z **Windows** (Nastavenia → Dáta), bez neho by som len hádal.
