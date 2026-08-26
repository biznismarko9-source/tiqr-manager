# TIQR Manager 2.0.58 — späť na modrú (2.0.56 zrušené)

## Čo si mi napísal

*"okej, teraz treba vratit modru farbu dashboradu, ta hneda nieje dobra"*

Spresnili sme si, že ide o celú appku naspäť na modrú (nie len Dashboard) — appka totiž farbí úplne
všetko cez jeden zdieľaný súbor, takže "len Dashboard modrý, zvyšok hnedý" by appku rozdelilo na dva
nekonzistentné vzhľady naraz, čo appka nikde inde nerobí.

## Čo je nové

Presný opak 2.0.56: `brand` (akcentová farba — tlačidlá, odkazy) je späť na modrú, `slate` (pozadia,
okraje, väčšina textu) je späť na sivo-modrú. Light mód je opäť biely s modrým akcentom, Dark mód opäť
tmavomodrý/navy — presne ako appka vyzerala pred 2.0.56, naprieč úplne všetkými stránkami (Dashboard,
Events, Orders, Sales, Tickets, Inventory, Pulls, Settings) naraz, keďže ide o ten istý jeden súbor
(`tailwind.config.js`), čo v 2.0.56 zmenil všetko na hnedú.

## Jedna vec, čo treba vedieť (nie je to 100% pixel-presné)

Appka si nikde neukladala celú pôvodnú modrú paletu (11 odtieňov pre `brand`) — v kóde aj v starom
reporte sa zachovali len 3 konkrétne odtiene z pôvodnej modrej škály (najsvetlejší `#eef4ff`, hlavný
akcent `#4a68f7`, najtmavší `#181c4d`), nie všetkých 11. Zvyšných 8 odtieňov som preto dopočítal
(plynulý prechod medzi tými 3 známymi bodmi), nie obnovil zo zálohy — najsvetlejší, hlavný a najtmavší
odtieň modrej sú teda presne také, aké appka mala predtým, tie medzi nimi sú veľmi blízke, ale nemusia
byť do posledného pixelu identické.

`slate` (sivo-modrá) je naproti tomu obnovená presne, bez dopočítavania — appka predtým používala
štandardnú farebnú paletu, ktorú má nástroj na štýlovanie (Tailwind) vstavanú, takže som ju len vrátil
naspäť v jej presnej, oficiálnej podobe.

Ak ti po nainštalovaní niektorý konkrétny odtieň príde vidieteľne iný, ako si pamätáš (napr. nejaké
tlačidlo pôsobí trochu inak), napíš mi presne kde — je to doladenie jedného čísla v jednom súbore, nie
veľká zmena.

## Ako som to overoval

```
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Keďže ide čisto o farby (žiadny Rust súbor, žiadna zmena logiky), `cargo test --lib` som nespúšťal znova
— nič, čo tie testy pokrývajú, sa nezmenilo. Namiesto vizuálneho renderu appky (čo v tomto prostredí
nejde) som:

- Prepočítal kontrast (čitateľnosť textu) podľa oficiálneho webového štandardu (WCAG) pre skutočné
  kombinácie, čo appka reálne používa (hlavný text, sekundárny text, odkazy/tlačidlá v Light aj Dark
  móde) — všetky prešli s rezervou.
- Skontroloval priamo skompilovaný výstupný súbor štýlov appky, že naozaj obsahuje nové modré farby a
  že staré hnedé nezostali nikde po starom builde.

## Čo teraz urobiť

1. Nainštaluj 2.0.58.
2. Prejdi si appku naprieč stránkami v oboch režimoch (Light aj Dark) a skontroluj, že modrá vyzerá
   dobre všade.
3. Ak niekde niečo nesedí, napíš mi presne kde.

## Zmenené súbory

**Frontend (1 súbor):**
- `tailwind.config.js` — `brand` a `slate` vrátené z hnedej/béžovej naspäť na modrú/sivo-modrú

**Verzia (8 miest):** ako vždy, všetkých na `2.0.58`.

## STOP

2.0.58 hotové — appka je späť na modrej. Skontroluj podľa krokov vyššie, hlavne či ti sedia jednotlivé
odtiene modrej (viď poznámka o dopočítaných odtieňoch vyššie).
