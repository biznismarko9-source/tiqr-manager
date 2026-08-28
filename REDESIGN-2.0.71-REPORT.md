# TIQR Manager 2.0.71 — Nová registrácia teraz čaká na tvoje schválenie

## Čo si vybral

Na otázku, ako presne má fungovať "pending approval", si vybral **reálne blokovanie** — nový účet sa naozaj
nedostane do appky, kým ho neschváliš, kontroluje sa to pri každom prihlásení (nie len raz hneď po
registrácii), a schvaľuješ priamo vo Firebase konzole (žiadna nová obrazovka v appke navyše).

Dôležité vopred: **tvoj vlastný účet ani nikto, kto sa dnes vie prihlásiť, sa touto zmenou nič nezmení.**
Kontrola sa týka len účtov založených od tejto chvíle ďalej.

## Ako to teraz funguje

Niekto sa zaregistruje (emailom, alebo cez "Continue with Google" prvý krát) → appka mu hneď ukáže obrazovku
**"Account pending approval"** namiesto appky samotnej, s tlačidlom Log out. Ty si niekedy potom otvoríš
Firebase konzolu, nájdeš jeho záznam, prepneš jedno políčko z `false` na `true`. Pri jeho ďalšom prihlásení
(alebo ak appku len nechá bežať a skúsi znova) sa mu už zobrazí appka normálne.

## Čo musíš urobiť TY, raz, aby to začalo fungovať

Kým toto neurobíš, appka nikoho nezablokuje ani nepustí navyše — nové registrácie ostanú jednoducho visieť
na "pending approval" natrvalo (bezpečný smer zlyhania), kým toto nedokončíš. Trvá to asi 3 minúty:

1. Otvor [Firebase konzolu pre tento projekt](https://console.firebase.google.com/project/tiqr-manager-b890a/firestore) (prihlás sa rovnakým Google účtom, cez ktorý appku spravuješ).
2. Ak vidíš tlačidlo **"Create database"**, klikni naň. Zvoľ **Production mode** (nie "Test mode"). Pri
   výbere regiónu je jedno, ktorú zvolíš — pokojne najbližšiu Európe (napr. `eur3 (europe-west)`).
3. Otvor záložku **Rules** (hore). Zmaž, čo tam je, a vlož presne obsah súboru `firestore.rules`, ktorý je
   súčasťou tohto balíčka (v koreňovom priečinku, vedľa `package.json`). Klikni **Publish**.
4. Hotovo — appka odteraz vie zapisovať aj čítať presne to, čo potrebuje, a nič iné.

Tento krok sa **neopakuje pri každej appke** — je to nastavenie projektu vo Firebase, urobíš ho raz.

## Ako schváliť nový účet

1. Vo Firebase konzole choď na **Firestore Database → záložka Data**.
2. Otvor kolekciu **users** — uvidíš jeden riadok na každého, kto sa niekedy zaregistroval. Riadky sú
   pomenované technickým ID, nie menom — klikni naň a v poliach vpravo uvidíš jeho **name** a **email**, takže
   vieš, koho schvaľuješ.
3. Nájdi pole **approved**, klikni naň a zmeň hodnotu z `false` na `true`. Uloží sa to hneď.

Žiadne tlačidlo "Approve" - len toto jedno pole. Ak sa ti to bude zdať nepohodlné časom (napríklad budeš
appku dávať viacerým ľuďom naraz), druhá možnosť z mojej otázky (schvaľovanie priamo v appke, s tlačidlom)
sa dá dorobiť neskôr — povedz.

## Čo som overil, a čo som overiť NEMOHOL

```
cargo test --lib   -> 693 testov, 0 zlyhaní, 3 ignorované (nedotknuté - táto zmena je len frontend)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Toto je jediná časť appky, ktorú som nemohol reálne odskúšať naživo — nemám prístup k tvojmu Firebase
projektu (ani by som ho nemal chcieť). Overil som teda len to, čo sa dá overiť bez neho: appka sa správne
skompiluje, typy sedia, a logika (kód, ktorý rozhoduje kedy niekoho pustiť/nepustiť) som si niekoľkokrát
prešiel ručne pre všetky prípady — starý účet, nový účet schválený, nový účet čakajúci, Firestore ešte
nenastavené, výpadok siete. Odporúčam ale: hneď po tomto kroku over si to sám jedným skúšobným
zaregistrovaním (napr. vlastný druhý email) — uvidíš "pending approval", schváliš ho podľa návodu vyššie,
over že sa potom naozaj dostane dnu.

## Zmenené súbory

**Frontend:**
- `src/lib/firebase.ts` — nový export `db` (Firestore).
- `src/lib/auth.tsx` — nové pole `approved` v auth kontexte; `register`/`loginWithGoogle` zapisujú nový
  účet ako čakajúci; nová `fetchApproved` (rieši aj starý účet bez záznamu, aj zlyhané čítanie).
- `src/pages/PendingApproval.tsx` — nová obrazovka.
- `src/App.tsx` — `RequireAuth` zobrazí `PendingApproval` namiesto appky, kým `approved` nie je `true`.

**Nové:**
- `firestore.rules` (koreňový priečinok) — pravidlá, ktoré vložíš do Firebase konzoly (krok 3 vyššie).

**Verzia (8 miest):** `2.0.71`.

## STOP — potrebujem od teba potvrdenie

Toto je jediný bod v appke, ktorý reálne závisí od kroku, čo musíš urobiť TY mimo appky (Firebase konzola) —
kým to nespravíš, nové registrácie ostanú navždy na "pending approval" (nikoho to nepustí dnu navyše, len
nikoho nepustí vôbec, kým to nedokončíš). Daj mi vedieť, keď to nastavíš a vyskúšaš skúšobnou registráciou,
nech viem, že to naozaj sedí aj naživo.
