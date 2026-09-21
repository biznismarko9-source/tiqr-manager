# TIQR Manager 2.47.1 — dátum, Ks a rozpoznávanie mien

Tri veci z tvojej správy. Pri dvoch som našiel presnú príčinu, pri tretej som
opravil, čo sa opraviť dalo — a na konci mám jednu otázku, bez ktorej by som
už len hádal.

---

## 1. Dátum nebol pokazený, bol orezaný

Toto bola dobrá chyba na nájdenie.

Kalendár sa otváral ako `position: absolute`. Lenže sedí vnútri tabuľky, ktorá
má `overflow: auto` — a **taký kontajner absolútne umiestnené okno oreže**.
Takže sa kalendár otvoril, len do neviditeľna.

Preto to „nešlo" naraz pri **order, pull aj event** — je to jeden komponent,
bola to jedna chyba.

Teraz je panel `position: fixed` a počíta si vlastné súradnice z pozície
políčka. Rovnako ako predtým sa preklopí nahor, keď dole nie je miesto, a
potiahne sa doľava pri pravom okraji. Rolovanie alebo zmena veľkosti okna ho
zavrie — lebo `fixed` panel sa s políčkom neposúva.

## 2. Ks: šípky preč, políčko väčšie

Presne ako na tvojej fotke: `type="number"` kreslí tie šípky **dovnútra**
políčka a pri 56 px sadli rovno na číslo.

Teraz je to obyčajné textové políčko, ktoré prijíma **len číslice**, bez
akýchkoľvek šípok — a širšie:

| | Predtým | Teraz |
|---|---|---|
| Inventory · Ks | 56 px | **72 px** |
| Pulls · Ks | 50 px | **70 px** |

Šírku som vzal z Poznámky, takže rozpočet 1112 px stále sedí presne a nič sa
nezmenšilo tak, aby to vadilo.

## 3. AI import

Našiel som tri konkrétne veci, ktoré boli naozaj zlé, a opravil ich.

### a) Meno eventu a platformy sa netrafilo

Porovnávalo sa **presne, znak po znaku**. Takže:

- fotka „Karpatské Chalupy 2026" → event „Karpatské Chalupy" **nenašlo**
- fotka „TICKETPORTAL.SK" → platforma „Ticketportal" **nenašlo**
- fotka „karpatske chalupy" (bez diakritiky) → **nenašlo**

AI to pritom prečítala správne. Formulár to len zahodil a nechal pole prázdne
bez vysvetlenia. Toto je podľa mňa veľká časť toho, čo si myslel tým, že
„nefunguje".

Teraz sa ignoruje diakritika, veľkosť písmen aj medzery navyše, a skúsi sa aj
začiatok a obsiahnutie. **Keď sedia dve možnosti, nevyberie ani jednu** —
tipovať medzi dvoma eventmi je horšie než prázdne pole, lebo zle trafený event
vyzerá správne a pritom zapíše lístky na iný večer.

Otestované na 12 prípadoch, všetky sedia.

### b) Dátum nákupu z fotky sa zahadzoval

AI ho z účtenky čítala **vždy** — formulár ho ignoroval a dal dnešný dátum.
Už ho berie.

### c) Skupiny (to z 2.47.0)

Zostáva, ako bolo: každá skupina zo screenshotu = jeden riadok, a o objednávkach
rozhoduje to isté pravidlo ako pri ručnom písaní.

---

## Čo som overil

- **Rozpoznávanie mien** prepísané do Pythonu a spustené na **12 prípadoch** —
  vrátane nejednoznačných („Ticket" pri dvoch platformách → nevyberie nič)
  a krátkych názvov.
- **Zoznam súborov** oproti 2.47.0: **0 pribudlo, 0 zmizlo, 4 zmenené.**
- **Zátvorky** proti 2.47.0 — identické vo všetkých štyroch.
- **Po prepísaní kalendára nezostal žiadny mŕtvy kód** (staré `above` /
  `alignRight` sú preč, strojovo overené).
- **Žiadny nepoužitý import.**
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a samotné volanie AI
— nemám ako ho odtiaľto spustiť.

---

## Jedna otázka, aby som ďalej nehádal

Pozrel som celú cestu AI importu a **tvar požiadavky je správny** pre model,
ktorý sa používa. Takže potrebujem vedieť, čo presne vidíš. Sú dve možnosti a
každá znamená niečo úplne iné:

1. **„AI import isn't available in this build."**
   → V builde nie je zašitý `ANTHROPIC_API_KEY`. To sa nastavuje ako secret
   v GitHube a **nie je to oprava v kóde** — vtedy AI nemôže fungovať nikdy,
   nech by bol kód akokoľvek správny.

2. **„AI analysis failed. Try again."**
   → Volanie odišlo a zlyhalo (kľúč, sieť, odpoveď modelu). Vtedy to viem ďalej
   rozobrať.

3. **Žiadna chyba, polia sa vyplnia — ale zle / prázdne.**
   → To je presne to, čo som opravil v bode 3a a 3b. Skús túto verziu.

Napíš mi, ktoré z toho to je (alebo pošli screenshot tej hlášky), a dorobím to.

---

**Verzia:** 2.47.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
