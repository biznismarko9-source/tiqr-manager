# TIQR Manager 2.46.0 — vyplňovače vedia presne čo je zle, a graf má os

Dve veci:

> „to nove vyplnovanie textov urobme to modernejsie a presnejsie, urobme to
> viac profesionalne"

> „ten graf ktory tam je tak nech uz tam vidno aj nejaky ten graf nieze on je
> prazdny nech tam je od nejakej po nejaku dobu"

---

## 1. Chyba svieti na políčku, nie vo vete dole

Doteraz ti formulár povedal **prvú** chybu ako vetu dole a o zvyšku mlčal.
Opravil si ju, klikol znova, dozvedel sa druhú. Pri ôsmich riadkoch je to osem
kôl.

Teraz **každé zlé políčko zčervenie naraz** — tým istým červeným krúžkom, aký
má zvyšok appky (nič nového som nekreslil) — a dole je napísané, koľko vecí
treba opraviť a od ktorého riadku začať.

### Kedy to zasvieti

Toto je tá časť, na ktorej záleží:

- **Napísal si niečo zle** — cena `201,0x`, dátum, čo nie je dátum: **hneď**.
  Je to zlé už teraz a pozeráš sa na to.
- **Niečo len chýba** — prázdny názov eventu, prázdna cena: **až keď klikneš
  Vytvoriť**. Prázdny formulár ešte nie je chybný a nemá ťa privítať červenou.

Tlačidlo Vytvoriť som **nechal aktívne**. Zhasnuté tlačidlo bez vysvetlenia je
horšie ako tlačidlo, ktoré ti po kliknutí ukáže, čo mu chýba.

## 2. Čo to robí rýchlejším

- **Čísla riadkov** — keď ti povie „riadok 3", nájdeš ho bez počítania.
- **Hlavička zostáva** pri rolovaní. Dvadsaťmiestne turné si drží názvy stĺpcov.
- **Duplikovať riadok** — nová ikonka vedľa ×. Šesť miest toho istého turné
  alebo štyri lístky, čo sa líšia len sedadlom, sú rýchlejšie skopírované než
  prepísané. Kópia vždy vyčistí to jedno pole, ktoré sa musí líšiť (sedadlá pri
  objednávke, meno pri evente, pre koho pri pulle).
- **Kurzor ide do nového riadku**, keď ho pridáš. Žiadna cesta myšou späť.
- **⌘↵ / Ctrl+↵ vytvorí** odkiaľkoľvek z formulára. Obyčajný Enter som nechal
  na pokoji — v dátumovom políčku a v rozbaľovačke už niečo znamená.

**Pri Sales duplikovanie nie je**, a to naschvál: riadok tam je **skutočný
lístok**, nie údaj. Nedá sa skopírovať niečo, čo existuje raz.

## 3. Ks a sedadlá si už neodporujú

Pri písaní validácie som narazil na skutočnú chybu — vlastnú.

Nová kontrola hlásila „2 sedadlá, ale 1 kus". Lenže **počet kusov má default
`1`** a keď napíšeš sedadlá, appka ich aj tak uprednostní. Takže napísať
`14-15` do čerstvého riadku je **normálne, správne použitie**, ktoré by som ti
označil za chybu.

Kontrolu som zrušil a namiesto nej políčko Ks teraz **ukáže počet zo sedadiel
a nedá sa prepísať**. Predtým tam ticho ostávala jednotka, ktorú appka
ignorovala. Nemôžu si odporovať, lebo je to jedno číslo.

## 4. Finance: graf už nie je bodka

Toto bola tvoja druhá vec a mal si pravdu.

Default obdobie je **„This month"**. Graf bucketoval po mesiacoch. Jeden mesiac
= **jeden bod**. Preto vyzeral prázdny — nebol prázdny, bol to jeden bod.

Teraz sa veľkosť kroku riadi podľa obdobia:

| Obdobie | Krok | Koľko bodov |
|---|---|---|
| Today | deň | 1 |
| **This month** | **deň** | **~30** |
| This year | mesiac | 9 |
| Custom do 92 dní | deň | až 92 |
| Custom nad 92 dní / All time | mesiac | až 24 |

Je to **tá istá logika, akú má graf na Dashboarde** — o to si žiadal minule.

### Čo som zámerne neurobil

**Nerozťahujem graf mimo zvoleného obdobia**, len aby bola čiara dlhšia. Dáta
prichádzajú už prefiltrované obdobím, takže mesiac mimo neho by sa nakreslil
ako **nula** — hoci si v ňom reálne obchodoval. To by bol graf, ktorý klame.

Okno **je** obdobie; mení sa len veľkosť kroku. Prázdne dni **vnútri** obdobia
sú naozajstné nuly — v ten deň sa nič nezaúčtovalo — a práve tie dávajú čiare
os, po ktorej beží.

„Today" je stále jeden bod. Jeden deň **je** jeden bod; to nie je chyba.

---

## Čo som overil

- **Bucketovanie som prepísal do Pythonu a spustil** na ôsmich prípadoch:
  „This month" → 20 denných bucketov (3 zápisy, 2 neprázdne), „This year" → 9
  mesačných, „Today" → 1, All time bez zápisov → prázdno, 3,4 roka → strop 24,
  92-dňové custom → dni, 112-dňové → mesiace. **Súčty sa zachovali** a sčítanie
  dní cez koniec mesiaca aj roka sedí (`2026-12-31 +1 = 2027-01-01`).
- **Zoznam súborov** oproti balíku 2.45.0: **0 pribudlo, 0 zmizlo, 7 zmenených**.
- **Vyváženosť zátvoriek aj JSX značiek** proti 2.45.0, súbor po súbore —
  **identická** vo všetkých siedmich.
- **Každý import sa dá dohľadať** v exporte druhého súboru; žiadny nepoužitý.
- **Verzia na 9 miestach v 7 súboroch**, všetky posunuté.

**Čo som pri tom sám našiel a opravil:** slovenská úvodzovka `„` v chybovej
hláške bola zatvorená rovnou `"`, čím v súbore ostal nepárny počet úvodzoviek
(v kóde neškodné, ale kontrola to hlásila) — a blokový komentár, ktorý mi sedel
medzi atribútmi JSX značky, som prepísal na riadkový.

**Čo overiť nedokážem:** preklad (Node ani Rust na tomto Macu nie sú, prvý
reálny build je až CI) a ako to sadne oku pri tvojich reálnych dátach.

---

**Verzia:** 2.46.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
