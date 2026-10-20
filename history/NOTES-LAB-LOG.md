# NOTES LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/7VGTbN6y8rrEpeHtzqMZAE
Sesterské denníky: [DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md) ·
[SETTINGS-LAB-LOG.md](SETTINGS-LAB-LOG.md) · [FINANCE-LAB-LOG.md](FINANCE-LAB-LOG.md)

---

**01 — Vzhľad je uzavretý, a je to ten dok.**
Vybral si si Look panel z finance labu a povedal: takto, všade —
dashboard, inventory, sales, finance, notes. Takže tento súbor začína
tým, že ten panel **zapíšem ako skutočný design system** a ďalej z neho
staviam úplne všetko.

**02 — THE REGISTER, dvanásť častí.**
1. *Popisok* — malý, utlmený, polotučný, **vždy NAD** svojím ovládačom,
   nikdy vedľa.
2. *Field* — popisok naskladaný nad tým, čo ovláda.
3. *Bar* — fieldy s medzerami 16 × 28 px, presne ako v doku.
4. *Segmented control* — **zapustená dráha, vybraná položka vystúpená
   ako plná pilulka**.
5. *Swatch* — 32 px štvorec, 5 px rádius, vybraný má biely rám.
6. *Panel* — povrch, vlasová linka, 8 px.
7. *Button* — rovnaké váhy a rádiusy ako v doku.
8. *Pill* · 9. *Input* (zapustený, ako dráha) · 10. *Row* ·
11. *Obsah poznámky* · 12. *Kostra stránky*.

Jeden akcent, len na výber. Žiadne tiene, žiadne prechody, žiadne
ilustrácie.

**03 — Takže návrhy sa líšia štruktúrou, nie štýlom.**
Žiadny návrh tu si nevymýšľa písmo, farbu, rádius ani ovládač. Tá fáza
skončila. Čo ostáva, je tvar obrazovky a to, čo sa rozhodne ukázať — čo
je aj tak ťažšia a užitočnejšia os.

**04 — Desať návrhov.**
Two Pane (zoznam + editor) · **Search Surface** (stránka sa otvára
prázdna; výsledky sú **bloky, nie poznámky**, a dajú sa upraviť rovno
vo výsledku) · Board (karty, pripnuté dopredu) · Outline (všetko zbalené
na jeden riadok, nadpisy sekcií vedľa) · One Note · **Bound Notes**
(poznámka pripnutá na event alebo objednávku; nepripnuté visia v skupine
dole, aby ticho nezmizli) · **Capture Forms** (šablóny + *run mode*:
320 px široký pás len s nezaškrtnutými položkami, zaparkuješ ho vedľa
prehliadača) · **Mentions Rail** (odkazy a **nepripojené zmienky** —
obyčajné hľadanie reťazca, nič sa nedomýšľa) · **Written Log** (jednotka
je *deň písania*, a hovorí na ktorom počítači si to písal) ·
**Review Deck** (jedna poznámka naraz a nad ňou dôvod, prečo vyplávala).

**05 — Dvanásť funkcií, najsilnejšie hore.**
1. **Pin to Record** — poznámka prestane byť voľný dokument. Osem
   z ostatných jedenástich dáva zmysel až vďaka tejto.
2. **Quick Note** — jedna klávesa kdekoľvek v apke, riadok, Enter, si
   späť na tom istom riadku. Ak nič nenapíšeš, nič z toho ďalej neplatí.
3. **Real Checklist** — položky s vlastným stavom a termínom; „4/7" sa
   ukáže na evente a nesplnené vyskočia v Attention.
4. **Conversation Log** — pripájacie zápisy s časom, kým a kanálom.
   Opravy ako nový zápis, nikdy tichá zmena.
5. **Bring It Back** · 6. **Find Any Note** · 7. **Event Day Sheet** ·
   8. **Keep Both Versions** (pri konflikte sa nevyberá víťaz) ·
   9. **Note Templates** · 10. **Evidence Shelf** · 11. **Standard
   Texts** · 12. **Linked Notes**.

Jedno, čo tam zámerne nie je: ukladanie hesiel. Tie patria do password
managera a apka to má povedať, nie ponúknuť políčko.

---

## Otvorené

- Ktorý návrh a ktoré funkcie.
- Audit skutočného kódu Notes — agent spadol na limite, nedobehol.
- Potom ten istý register aplikovať na dashboard, inventory, sales,
  finance.
