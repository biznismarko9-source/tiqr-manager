# TIQR Manager 2.49.1 — to 403 bolo chýbajúce povolenie

Tá chyba, čo si poslal, je zlatá. Presne na toto bolo dobré to, čo som pridal
v 2.48.0 — konečne vidím, čo sa deje.

---

## Čo tá chyba hovorí

```
403  "Request had insufficient authentication scopes"
     reason: insufficientPermissions
```

To **nie je** vypnuté API a **nie je** to expirovaný token. Znamená to jednu
jedinú vec: **tvoj prihlasovací token nemá povolenie na Google Drive.**

Overil som pritom aj to, čo by bolo podozrivé:

| Kontrola | Výsledok |
|---|---|
| Pýta si appka vôbec `drive.file`? | **áno**, je v zozname povolení |
| Posiela `prompt=consent`, aby sa dalo dopýtať znova? | **áno** |
| Posiela `access_type=offline`? | **áno** |

Takže kód bol v poriadku. Chybu mal inde.

## Ako sa to stalo

Na tej Google obrazovke s povoleniami má **každý riadok vlastné zaškrtávacie
políčko**. Keď jedno odškrtneš (alebo ti tam pribudne nové a ty ho preklikneš),
Google ti aj tak vráti **úplne platný token** — len bez toho povolenia. A spolu
s ním pošle zoznam toho, čo naozaj povolil.

**Appka sa na ten zoznam nikdy nepozrela.** Uložila taký token ako úspešné
prihlásenie, napísala „Signed in as ..." — a každé ďalšie volanie na Drive
padlo na 403. Donekonečna, bez stopy, bez vysvetlenia.

To je tá diera. Prihlásenie vyzeralo, že prešlo, a pritom bolo od začiatku
nepoužiteľné.

## Čo je opravené

**1. Kontroluje sa, čo Google naozaj povolil.** Ak pri prihlásení chýba Drive
(alebo Sheets), **prihlásenie rovno zlyhá a napíše ktoré povolenie chýba** —
namiesto toho, aby sa uložil polovičný token a lámalo sa to až o päť minút.

Porovnáva sa len tie dve „API" povolenia a podľa kusu textu, nie celý reťazec.
Má to dôvod: `openid` sa vracia ako `openid`, ale `email` sa vracia ako
`https://.../auth/userinfo.email` — doslovné porovnanie by hlásilo chybu pri
**každom** úspešnom prihlásení.

A keď Google zoznam nepošle vôbec, **neberie sa to ako odmietnutie**. Zhodiť
prihlásenie kvôli údaju, ktorý jednoducho neprišiel, by bolo horšie než tá
pôvodná chyba.

**2. Tá hláška ťa posielala na zlé miesto.** Začínala tým, že „Drive API nie je
zapnuté v Google Cloud projekte" — a poslala ťa do konzoly za problémom, ktorý
tam nebol. Teraz sa číta odpoveď od Googlu a rozlíšia sa tri rôzne veci, ktoré
predtým zdieľali jednu vetu:

- **`insufficientPermissions`** → prihlás sa znova a nechaj **všetko** zaškrtnuté
- **`accessNotConfigured`** → naozaj treba zapnúť API, tu je odkaz
- **401** → vypršané prihlásenie (dostalo vlastnú vetu, nikdy nemalo byť spolu s 403)

---

## Čo máš spraviť ty

**Settings → Integrations → prihlásiť sa Googlom znova**, a na tej Google
obrazovke **nechať zaškrtnuté všetky políčka**, hlavne to o Drive.

Ak niektoré odškrtneš, appka ti to **teraz rovno povie** a nepustí ťa ďalej
s pokazeným prihlásením.

## Čo som overil

- **Štyri nové unit testy** na to porovnávanie povolení: Drive odškrtnutý sa
  odmietne a **pomenuje sa len Drive** (nie Sheets); chýbajúci zoznam prihlásenie
  nezhodí; prihlásenie „len identita" (to tlačidlo Continue with Google) sa
  nikdy nemeria proti Sheets/Drive; všetko povolené = nič nechýba.
- **Zátvorky v oboch Rust súboroch** proti 2.48.1 — **bez posunu**.
- **Ten jediný skutočný literál `TokenResponse`** má po pridaní poľa všetkých
  päť položiek — strojovo porovnané proti definícii štruktúry.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Rust tu nie je, prvý reálny build je CI) ani
samotné prihlásenie do Googlu.

---

**Verzia:** 2.49.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.
