# TIQR Manager 2.0.82 — Price Checker: rýchlejšie zadávanie cien

> *"ja to chcem tak, aby som tam dal len link a dashboard to cez ai cele preskumal a porovnal ceny"*

## Najprv, prečo to nie je presne takto

Toto sme si overovali ešte pred 2.0.81 a znova teraz — nič sa v tom nezmenilo, aj keď som to čerstvo
preveril na webe. StubHub nemá pre bežného predajcu jednoduchý prístup (aj ich oficiálne API dostaneš len
tak, že napíšeš ich support tímu a schvália ťa, a aj vtedy je to na spravovanie TVOJICH vlastných ponúk, nie
na sledovanie cudzích cien) a dosť intenzívne investujú do blokovania automatického sťahovania svojich
stránok. Vivid Seats má API len cez ich broker platformu pre schválených predajcov. Ticombo nemá verejné API
vôbec. A "cez AI" na tom nič nemení — appka (alebo AI), čo by si sama otvorila tú stránku a čítala z nej
ceny, je z pohľadu tých stránok stále automatizovaný prístup bez teba, presne to, čo StubHub aktívne
blokuje a čo porušuje ich podmienky. Takže to zostáva mimo, presne ako v 2.0.81.

## Čo je nové namiesto toho

V okienku "Check Prices" (na každej marketplace karte) pribudlo úplne hore nové políčko: **"Paste from the
listings page"**. Otvoríš si stránku StubHub/Vivid Seats/Ticombo sám ako doteraz, označíš tam ceny, Ctrl+C,
a vložíš ten kus textu do tohto nového políčka. Appka si z neho sama vytiahne najnižšiu, priemernú, najvyššiu
cenu a počet listingov — a rovno ich vyplní do tých istých 4 políčok, čo si doteraz prepisoval ručne jedno po
druhom. Vie rozoznať aj menu (€, $, £, Kč, zł, Ft a ďalšie) a podľa nej prepne dropdown, ale iba keď je to
jednoznačné — ak text spomína viac mien naraz, appka radšej nehádže a necháš si vybrať menu sám.

Ak appka v texte nenájde žiadne ceny (napr. si tam vložil niečo iné), nič sa neprepíše — pôvodné hodnoty
zostanú tak, ako boli, a napíšeš to ručne, presne ako doteraz. Všetkých 5 políčok (4 čísla + mena) zostáva aj
po vložení plne editovateľné — appka len navrhne čísla načítané z toho, čo si skopíroval, ty si ich pred
uložením skontroluješ a prípadne opravíš.

Appka sama nikam nechodí ani teraz — stále ty osobne otváraš tú stránku a kopíruješ z nej text. Toto ti len
ušetrí prepisovanie čísel, ktoré už máš pred očami.

## Ako presne to funguje pod kapotou

Nová čisto-frontendová funkcia (`src/lib/priceParse.ts`), žiadna zmena v databáze ani na backende. Vytiahne
z vloženého textu všetky čísla, čo vyzerajú ako ceny (uprednostní tie hneď vedľa symbolu/kódu meny, aby
neomylom nezobrala napríklad číslo sedadla alebo rok podujatia), a rozpozná aj čiarku/bodku ako desatinné
miesto podľa toho, čo dáva zmysel (funguje na americký aj európsky spôsob zápisu čísel).

## Čo som overil

Napísal som 13 testovacích prípadov na tú novú funkciu samostatne (americký zápis s $ a čiarkami po tisícoch,
európsky zápis s € a desatinnou čiarkou, obyčajný stĺpec čísel bez symbolu meny, text s rokom podujatia, čo sa
nesmie omylom zobrať ako cena, text s viacerými menami naraz) — pri prvom behu to odhalilo skutočnú chybu
(pri "Rad 200 $99" appka omylom priradila symbol $ k číslu 200 namiesto k 99), opravil som to a všetky testy
teraz prechádzajú. Potom som si to ešte reálne odklikal v prehliadači (vlastný dočasný testovací setup, zmazaný
po overení) — vloženie textu, automatické vyplnenie polí, uloženie kontroly, aj správanie keď appka nič
nenájde — všetko fungovalo bez jedinej chyby v konzole.

```
cargo test --lib   -> 765 testov, 0 zlyhaní, 3 ignorované (nezmenené - žiadny Rust súbor sa nemenil)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.0.82 build" v hlavičke)
```

## Zmenené súbory

**Frontend:**
- `src/lib/priceParse.ts` — nový súbor, celá logika na vytiahnutie cien z vloženého textu
- `src/pages/PriceChecker.tsx` — nové políčko v "Check Prices" okienku, napojené na tú novú funkciu

Žiadny backend súbor, žiadna migrácia, žiadna zmena v databáze.

**Verzia (9 miest v 7 súboroch):** `2.0.82`.

## STOP

2.0.82 hotové, otestované a zabalené. Skontroluj:

1. V sidebari **Price Checker** → vyber ľubovoľný event.
2. Na niektorej marketplace karte klikni **"Check Prices"**.
3. Do nového políčka "Paste from the listings page" skús vložiť niečo ako `$120  $135  $99  $150` (alebo
   rovno skutočné ceny skopírované z reálnej stránky) — polia nižšie by sa mali samé vyplniť.
4. Skús vložiť text bez čísel (napr. "ahoj") — polia by mali zostať také, aké boli, a mala by sa objaviť
   krátka poznámka, že sa nič nenašlo.
5. Ulož a over, že sa to na karte objaví presne tak, ako pri ručnom zadaní doteraz.
