# TIQR Manager 2.109.1 — oprava: appka padala pár sekúnd po spustení syncu

> *„apka do par sekund od spustenia sync padne, je tam vazny problem"*
> *„nic nemen ako to funguje len to oprav"*

Opravené. Nič iné sa nemení — žiadna nová funkcia, žiadna zmena správania,
žiadna migrácia.

---

## 1. Príčina — nájdená a reprodukovaná, nie odhadnutá

Pri ukladaní každého nového mailu appka z predmetu a náhľadu číta počet
lístkov, sumu, číslo objednávky a OTP kód.

Čítanie **počtu lístkov** nájde slovo „tickets" / „lístky" a pozrie sa
**12 bajtov** pred neho, či tam nie je číslo. Lenže text sa neukladá po
písmenách, ale po bajtoch:

| znak | bajty |
|---|---|
| `a` | 1 |
| `š`, `é` | 2 |
| `€`, `–` | 3 |
| `🎟️` | 7 |

Keď tých 12 bajtov dozadu padne **doprostred** emoji alebo diakritiky, Rust
to odmietne a spanikári. Vydaná appka je nastavená tak, že pri panike sa
**celá zavrie** (`panic = "abort"`) — preto žiadna chybová hláška, len
zavreté okno.

Presne taký predmet: `🎟️ Your tickets for Coldplay are ready`. Sync sťahuje
po 40 mailoch a v prvých dávkach sa niečo také nájde skoro vždy — preto
**„do pár sekúnd"**.

2.108.0 opravila tú istú chybu pri čítaní **sumy** (`safe_slice`), ale toto
jedno miesto vynechala.

---

## 2. Tá istá chyba ešte na troch miestach — opravené naraz

| Kde | Kedy to padalo |
|---|---|
| čítanie počtu lístkov | **sync — toto bol ten pád** |
| skracovanie textu na 400 bajtov | sync (dlhší náhľad s diakritikou alebo emoji) a **otvorenie** dlhšieho mailu |
| čítanie HTML entít (`&amp;`, `&nbsp;`) | otvorenie HTML mailu so slovenským textom |
| výsledok vyhľadávania vo Workspace | hľadanie v poznámke so znakom ako `İ`, ktorý po zmenšení písmen zmení dĺžku |

Každá oprava robí to isté: rez sa posunie na **najbližší celý znak**, namiesto
toho, aby padol doprostred neho.

---

## 3. Čo sa NEzmenilo

Ako appka funguje. Pre každý text, ktorý predtým nepadal, vychádza **presne
to isté** — to isté číslo, ten istý text, to isté skrátenie. Zmena je len
tam, kde predtým appka spadla.

---

## 4. Ako je to overené — tentoraz naozaj spustené

2.109.0 sa tam, kde vznikla, nedala skompilovať — prvý kompilátor bolo CI.
Táto verzia áno: celý backend aj frontend sú **skompilované a spustené**
(Linux, cargo, npm).

- **7 nových testov. Pred opravou všetkých 7 padlo presne touto chybou**
  (`start byte index 1 is not a char boundary; it is inside '🎟'`), po
  oprave prechádzajú. Najprv reprodukcia, potom oprava.
- Jeden z nich skúša každý druh znaku (`é`, `–`, `€`, `🎟`, `İ`, …) na
  **každej pozícii** okolo **každého** slova, ktoré čítačky hľadajú —
  zhruba 41 000 textov cez 17 funkcií. Čítačka, ktorá reže podľa bajtov,
  týmto neprejde, nech je ktorákoľvek.
- Jeden ide **celou cestou**, ktorou sync ukladá mail: klasifikácia,
  uloženie so všetkými vyčítanými poľami, tvoje pravidlá a log — na
  skutočnej databáze so všetkými 45 migráciami.
- Celý Rust balík: **1404 testov, 0 červených.** Tých 6, ktoré boli
  červené v 2.109.0, už prechádza.
- Frontend: `npm run build` (TypeScript + Vite) prechádza.

**Čo overené nie je:** skutočný sync proti tvojej Gmail schránke — tá tu
nie je. Pád je ale reprodukovaný na presnom kóde, ktorým sync každý mail
ukladá, a po oprave tým istým kódom prejde.

---

## 5. Všimnuté, ale nezmenené — lebo „nič nemeň, len oprav"

Keď otvoríš mail, jeho telo sa zobrazí **najviac ako ~400 znakov a v jednom
riadku**: backend ho pošle cez to isté skracovanie ako náhľady
(`get_mail_body` → `safe_detail`). Nepadá to, len to ukazuje menej, než by
mohlo. Zapísané v `PROJECT_STATE/KNOWN_BUGS.md`; ak to chceš opraviť, je to
samostatná malá úprava.

---

## Súbory

**Zmenené:** `src-tauri/src/commands/mail.rs` (3 opravy + 6 testov),
`src-tauri/src/commands/workspace.rs` (1 oprava + 1 test)

**Verzia 2.109.1** v `package.json`, `package-lock.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`,
`1-CLICK-UPDATE.bat`, `release.ps1`. Nové číslo je nutné: 2.109.0 už je
vydaná a updater rovnaké číslo neponúkne.

**Ďalšia nová migrácia je stále 046.**
