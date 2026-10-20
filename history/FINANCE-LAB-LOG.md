# FINANCE LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/9mV8ceJGsTUZeRR5AynK1z
Sesterské denníky: [DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md) ·
[SETTINGS-LAB-LOG.md](SETTINGS-LAB-LOG.md)

Štyri záložky — Overview, Transactions, Accounts, Reports — a ku každej
desať návrhov.

---

**01 — Každý návrh má vlastný VZHĽAD, nie len vlastné rozloženie.**
Toto bolo to, čo ti vadilo na predchádzajúcich kolách: desať návrhov,
ktoré boli desať usporiadaní jedného vizuálneho jazyka. Tu má každý
návrh **kožu** — vlastné písmo, vlastné linky a výplne, vlastnú hustotu,
vlastný spôsob kreslenia čísla. Účtovná kniha v mono s vlasovými linkami
je iný objekt ako editoriálna dvojstrana v serife, a nemajú pôsobiť ako
súrodenci.

Koží je osem: Ledger, Editorial, Report, Board, Terminal, Quiet, Grid,
Soft. V doku sa dá jedna nasilu nasadiť na všetkých štyridsať — buď aby
si porovnal štruktúry bez vizuálneho šumu, alebo aby si videl, čo jeden
vzhľad urobí s každou obrazovkou.

**02 — Peniaze ostávajú celé centy.**
Každé číslo sa počíta v centoch a formátuje sa až na okraji, presne ako
to apka ukladá. Nikde pri súčte nie je desatinné číslo.

**03 — Nič tu nepredpovedá, neodhaduje ani si nevymýšľa.**
Každé číslo pochádza z toho, čo si zadal alebo naimportoval. Žiadne
projekcie, žiadne „očakávané" tržby, žiadne odhady trhu. Kde údaj nie je
známy, tam chýba — nedopĺňa sa.

**04 — Hotové: Overview a Transactions.**
Dvadsať návrhov, všetky funkčné a preklikateľné.

*Overview:* Ledger Line · One Number · Waterfall · **Payout Spine**
(rysovacia doska, meria najdlhšiu medzeru bez príjmu) · Terminal ·
**Profit Mosaic** (dlaždica na akciu, stratové šrafované) · Spread ·
**Three States** (zamknuté / na ceste / prišlo, v skutočnom pomere) ·
**Currency Desk** (čo ťa ticho stojí kurz) · **Margin Ribbon**
(seizmograf marže cez celý rok).

*Transactions:* **Bank Tape** (papierová páska s bodkovanými vodičmi) ·
**Grid Sheet** (tabuľka na prilepenie CSV) · **Sentence Ledger** (vety
namiesto stĺpcov) · **Match Column** (dva papiere, kreslené nite,
nespárované riadky s čiernym pruhom) · **Colour Field** (log stĺpce,
squint mode) · **Running Rail** (niť zostatku v okraji) · **Event
Folders** (zložky + zásuvka na siroty) · **Audit Trail** (odkiaľ riadok
prišiel) · **Entry Line** (jeden obrovský input) · **Voucher Pane**.

**05 — Hotové aj Accounts a Reports. Štyridsať návrhov, všetky funkčné.**

*Accounts:* **Ledger Spread** (viazaná kniha, dve protiľahlé strany,
dvojitá linka pod súčtom) · **Payout Board** (letiskové tablo, čisté
písmo, žiadny tvar) · **Capital Flow** (Sankey — hrúbka stuhy do
nepredaných lístkov proti tej, čo sa vracia do banky) · **Ageing Strip**
(geologický vrt, hrdzavý koniec = najstaršie peniaze) · **Balance Pans**
(rysovacia doska, váha sa reálne nakláňa) · **Stock Capital** (dlaždice
farbené podľa **veku peňazí**, nie zisku) · **Bank Reconcile** (dva
stĺpce, nite sa ťahajú rukou, rozdiel musí dôjsť na nulu) · **Account
Passbook** (vkladná knižka, stĺpec zostatkov je zároveň graf) · **Float
Gauges** (prístrojová doska, tmavý pás vnútri = už zadané peniaze) ·
**Card Runway** (pravítko, dĺžka fúzu = bezúročné dni).

*Reports:* **Closing Statement** (to, čo vytlačíš a pošleš) · **Profit
Bridge** (vodopád cez celú obrazovku, prepočítateľný na jednu platformu)
· **Fee Anatomy** (skutočná sadzba z tvojich vlastných predajov —
StubHub 16,1 %, Gigsberg 5,0 %) · **Contact Sheet** (dvadsať okienok,
značky robíš ty, nie appka) · **Margin Spread** (beeswarm, chvost strát
sa dá otvoriť) · **Currency Drift** (notová osnova: čo si zaúčtoval
proti tomu, čo naozaj prišlo) · **Tax Sheet** (úradný formulár, každý
riadok sa rozbalí na transakcie; nič neradí, len usporadúva) ·
**Broadsheet** (novinová strana, čísla vo vetách) · **Season Poster**
(jedno obrovské číslo) · **Return Staircase** (za koľko dní sa ti
peniaze vracajú — polovica za 31 dní, deväťdesiat percent za 68).

**06 — Čisté dopredu, výraznejšie za ne.**
Prvé kolo zašlo priďaleko. Každá záložka sa teraz otvára na **čistej
päťke** postavenej z toho, čo apka už má — panely, tabuľky, jeden
akcent, veľa vzduchu. Sú to stále päť rôznych štruktúr, len tiché.
Tých desať výraznejších ostáva na tom istom páse za nimi, takže sa nič
nestratilo a porovnanie je na jeden klik. Spolu **60 návrhov**.

*Čisté — Overview:* Summary (predvolená) · Lead Figure · Two Columns ·
Position · Month List.
*Transactions:* Table · By Day · List and Detail · Filtered · Compact.
*Accounts:* All Accounts · Account Cards · Held and Owed · What Is
Coming · Bank.
*Reports:* Profit and Loss · By Platform · By Event · Month by Month ·
Fees.

**07 — Dvanásť funkcií je v labe, na záložke Functions.**
Každá má **Try it** a **Add**. Try beží proti demo sade so zasadenou
chybou a vracia sa červená alebo oranžová — náhľad, čo sa vráti celý
zelený, ťa naučí, že funkcia nič nerobí. Keď dáš Add, funkcia sa
naozaj zjaví hore na tej záložke, kam patrí.

Poradie podľa toho, čo najviac bolí:

1. **Bank Match** — priloží bankový výpis k tomu, čo je v apke. Tri
   stĺpce: spárované, *riadky z banky, ktoré si nikto nenárokuje*, a
   *objednávky označené ako zaplatené, ku ktorým žiadne peniaze
   neprišli*. Demo nájde jednu za 740 € starú 51 dní.
2. **Waiting on Money** — čo je predané a ešte nie je na účte, starnuté
   proti tvojim vlastným pravidlám výplat. Rozlišuje meškanie od
   nevinnej príčiny (presunutý event).
3. **Nothing Hidden** — pod každým súčtom riadok „počítalo sa 386 z 391
   · 5 sa nepočítalo". Červený segment má **pevnú minimálnu šírku**,
   takže jeden vypadnutý riadok sa nikdy nevykreslí ako nič. Presne
   toto by bolo chytilo incident s € v prvý deň.
4. **Books Check** · 5. **What the Platforms Took** (skutočná sadzba a
   odľahlé predaje) · 6. **Event Result** (dva stĺpce výsledku: čo
   naozaj prišlo a čo by prišlo, keby všetko dobehlo) · 7. **Recorded
   Twice** · 8. **Cost Counted Twice** · 9. **Close the Month** ·
   10. **Two Laptops Check** · 11. **Money That Came Back** ·
   12. **What the Bank Says**.

---

## Otvorené

- Ktorý návrh na ktorej zo štyroch záložiek.
- Ktoré funkcie ideme reálne stavať a v akom poradí.
