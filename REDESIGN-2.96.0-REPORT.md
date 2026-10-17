# TIQR Manager 2.96.0 — Neon

Vybral si z labu **Neon** a jeho výrazné farby. Je nasadený na celý TIQR.

---

## 1. Čo sa zmenilo

| | pred | po |
|---|---|---|
| podklad | `#08090d` | **`#0a0618`** void |
| akcent | violet `#8b5cf6` | **fuchsia `#c026d3`** |
| panel | nepriehľadný `#11131b` | **sklo** `rgba(255,255,255,.05)` |
| linka | `#272938` | `rgba(255,255,255,.12)` |
| rádius | 12 px | **18 px** |
| pozadie | plochá farba | **dve fixné nasvietenia** |

Biela na `brand-600` meria **4,6 : 1**. To je najtesnejšia z troch paliet,
ktoré táto appka niesla — pred akoukoľvek zmenou plneného tlačidla to treba
premerať znova.

---

## 2. Prečo bolo treba zasiahnuť do ôsmich súborov

Sklo nie je efekt na paneli. Je to **priehľadnosť nad niečím**.

`.dark body` nesie dve fixné radiálne nasvietenia a každý panel nad nimi je
`rgba`. Lenže osem miest v appke maľovalo **nepriehľadné pozadie** cez ten
podklad — shell, Recap, Welcome, prihlasovacie stránky. Tie by nasvietenie
prekryli a appka by vyzerala len vyblednuto, nie sklenene.

Všetkých osem je teraz `dark:bg-transparent`.

---

## 3. Chyba, ktorú si tá plošná náhrada spravila

**Dve z tých ôsmich neboli pozadia stránok. Boli to stmavenia za dialógmi.**

`dark:bg-slate-950/70` → moja náhrada z toho spravila `bg-transparent/70`.
Čo je priehľadná farba pri 70 % — teda **nič**. Každý dialóg v appke by sa
vznášal nad živým obsahom bez stmavenia.

Opravené na `dark:bg-[#05030c]/70` a overené renderom.

Poznámka pre mňa aj pre teba: **plošná náhrada cez názov triedy je presne
taká bezpečná ako kontrola, ktorá po nej nasleduje.** Tu to zachránilo to,
že som si prešiel, čo presne sa zmenilo, namiesto aby som sa spoľahol na
počet zásahov.

---

## 4. Jedno rozhodnutie, ktoré som spravil proti efektu

**`backdrop-filter` nie je na `.card`.**

Každý panel v appke je `.card`. Samotné Settings ich vykreslia desiatky.
Blur znamená samostatný kompozičný prechod **na každý jeden prvok**, a ten
účet príde na notebooku, nie na mojom renderi.

Sklo pritom číta hlavne z **priehľadnosti nad nasvieteným podkladom**, a tá
je zadarmo. Blur je preto len tam, kde si ho panel naozaj zaslúži — na
`.card-glass` a na všetkom so `shadow-overlay`, teda na dialógoch a
menu, ktoré sa vznášajú nad **obsahom**, nie nad pozadím.

Ak ho chceš aj na bežných kartách, je to jeden riadok. Ale odmeraj to najprv
na tabuľke Inventory.

---

## 5. Dve veci, kde som uprednostnil čitateľnosť pred vzhľadom

**Tabuľky majú hustejšiu výplň než karty** (`bg-surface-muted`). Dvanásť
riadkov nad nasvieteným prechodom je naozaj horšie čitateľných — v labe som
to pri Neone napísal ako jeho cenu. Rieši sa to nepriehľadnosťou, nie tmavším
textom.

**Vedúce číslo svieti cez `currentColor`, nie cez gradient.** Zisk je zelený,
keď si zarobil, a červený, keď si stratil. Gradientová výplň by ten význam
prepísala. `.fig-glow` preto pridá svetlo **vo farbe, ktorú to číslo už má**.
Gradient (`.fig-lit`) ostáva pre čísla, ktorých farba nič neznamená.

---

## 6. Overenie

Vyrenderoval som to s **hodnotami skopírovanými priamo z kódu**: nasvietený
podklad, sklené panely, fuchsiový akcent na navigácii a pilulkách, graf s
magenta plochou, svetielkujúci zisk, čitateľná tabuľka — a zvlášť opravené
stmavenie dialógu.

Plus vyváženie zátvoriek celého `src/`, `index.css` 114/114,
`tailwind.config.js` 12/12, obe triedy komentárovej chyby 0.

Jednu vec som si ustrážil preventívne: komentár k tej žiare som najprv napísal
**medzi JSX atribúty**. Je to platné, ale tento súbor mi build zhodil už
dvakrát práve na umiestnení komentára (2.80.0, 2.86.0) — nemá zmysel
zisťovať, či to vyjde do tretice. Presunul som ho.

**Nie je to kompilácia.** CI je prvý kompilátor.

---

## 7. Verzia

2.95.0 → **2.96.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
