//! 2.113.1 - the few lines of an email that matter, picked out of its text.
//!
//! marko: "pri mailoch ... napr please send your tickets a tam je viac info
//! ako napriklad poslat tam, meno a mail a to daj ako hidden a ked na to
//! kliknes tak sa to zobrazi a aj cena, cize nie len prvy text ale aj realne
//! obsah a vybrat z toho najpodstatnejsie".
//!
//! So an email's own text - not Gmail's 200-character preview - is read for
//! where the tickets go and to whom, for how much, by when, for which event,
//! each line with the mail's own label. A person's details (a name, an
//! e-mail address, a phone number, an address) are marked `private`: TIQR
//! shows them on a click, Discord as a spoiler (commands/discord.rs).
//!
//! No AI and no guessing: a value is taken only from beside a label the mail
//! itself wrote ("Total Proceeds: €720.78", "Buyer email: ..."), or - for
//! where to send tickets - from a sentence that asks for exactly that. The
//! text is untrusted like everything in a message; it only ever comes out
//! again as text.
use serde::Serialize;

/// One line worth knowing at a glance.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MailFact {
    /// The mail's own label, tidied ("Total Proceeds", "Buyer's name").
    pub label: String,
    pub value: String,
    /// A person's details - hidden until clicked.
    pub private: bool,
    /// What the line is about: "sendTo", "name", "email", "phone",
    /// "address", "deadline", "money", "event", "when", "venue", "tickets",
    /// "seats" or "reference".
    pub kind: &'static str,
}

/// What a line is about, in the order the facts are shown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    SendTo,
    Name,
    Email,
    Phone,
    Address,
    Deadline,
    Money,
    Event,
    When,
    Venue,
    Tickets,
    Seats,
    Reference,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::SendTo => "sendTo",
            Kind::Name => "name",
            Kind::Email => "email",
            Kind::Phone => "phone",
            Kind::Address => "address",
            Kind::Deadline => "deadline",
            Kind::Money => "money",
            Kind::Event => "event",
            Kind::When => "when",
            Kind::Venue => "venue",
            Kind::Tickets => "tickets",
            Kind::Seats => "seats",
            Kind::Reference => "reference",
        }
    }
    fn private(self) -> bool {
        matches!(self, Kind::SendTo | Kind::Name | Kind::Email | Kind::Phone | Kind::Address)
    }
    /// How many facts of one kind are worth showing.
    fn most(self) -> usize {
        match self {
            Kind::Email | Kind::Money => 2,
            Kind::Seats => 3,
            _ => 1,
        }
    }
}

/// Labels as `fold` writes them: lower case, no accents, words only.
const LABELS: &[(Kind, &[&str])] = &[
    (
        Kind::SendTo,
        &[
            "send to", "send tickets to", "send the tickets to", "send them to", "transfer to", "transfer tickets to",
            "transfer the tickets to", "deliver to", "delivery to", "recipient", "recipient name", "poslat na",
            "poslat komu", "prijemca", "empfanger", "senden an", "destinatario", "enviar a",
        ],
    ),
    (
        Kind::Name,
        &[
            "name", "full name", "buyer", "buyer name", "buyers name", "customer", "customer name", "attendee",
            "attendee name", "ticket holder", "ticket holder name", "holder name", "guest name", "lead booker", "meno",
            "jmeno", "kupujuci", "kupujici", "kaufer", "comprador", "nombre", "titular",
        ],
    ),
    (
        Kind::Email,
        &[
            "email", "e mail", "email address", "e mail address", "buyer email", "buyers email", "recipient email",
            "attendee email", "customer email", "send to email", "transfer email", "mail", "correo",
            "correo electronico",
        ],
    ),
    (Kind::Phone, &["phone", "phone number", "mobile", "mobile number", "telephone", "tel", "telefon", "telefono"]),
    (
        Kind::Address,
        &["address", "shipping address", "delivery address", "postal address", "adresa", "adresse", "direccion"],
    ),
    (
        Kind::Deadline,
        &[
            "send by", "send before", "transfer by", "transfer before", "deliver by", "delivery deadline",
            "transfer deadline", "deadline", "ship by", "dispatch by", "must be sent by", "due by", "due date", "termin",
            "frist", "fecha limite", "odoslat do",
        ],
    ),
    (
        Kind::Money,
        &[
            "total proceeds", "proceeds", "net proceeds", "payout", "payout amount", "you will receive",
            "you ll receive", "youll receive", "you will be paid", "earnings", "net amount", "sale price",
            "selling price", "sold for", "total", "order total", "total paid", "total price", "total amount",
            "grand total", "amount", "amount paid", "price", "ticket price", "celkom", "spolu", "suma", "cena",
            "gesamt", "gesamtbetrag", "auszahlung", "erlos", "betrag", "total a pagar", "importe", "importe total",
            "precio", "prezzo", "totale",
        ],
    ),
    (
        Kind::Event,
        &[
            "event", "event name", "event title", "show", "concert", "match", "game", "performance", "artist",
            "podujatie", "akcia", "veranstaltung", "evento", "espectaculo",
        ],
    ),
    (
        Kind::When,
        &["date", "event date", "date time", "date and time", "when", "time", "show date", "match date", "datum", "fecha", "kedy"],
    ),
    (
        Kind::Venue,
        &["venue", "location", "where", "stadium", "arena", "miesto konania", "veranstaltungsort", "lugar", "recinto"],
    ),
    (
        Kind::Tickets,
        &["quantity", "qty", "number of tickets", "tickets", "ticket quantity", "no of tickets", "pocet", "pocet listkov", "anzahl", "cantidad"],
    ),
    (Kind::Seats, &["section", "row", "seat", "seats", "block", "sector", "sektor", "rad", "fila", "asiento"]),
    (
        Kind::Reference,
        &[
            "order number", "order no", "order", "order id", "order ref", "order reference", "sale id",
            "sale number", "sale no", "sale reference", "listing id", "listing number", "booking reference",
            "booking number", "booking id", "reference", "reference number", "confirmation number",
            "confirmation code", "transaction id", "ticket id", "request", "request number", "cislo objednavky",
            "objednavka", "bestellnummer", "auftragsnummer", "numero de pedido", "pedido", "n de pedido",
        ],
    ),
];

/// A label in comparable form: lower case, accents folded, words only.
fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mapped: &str = match c {
            'á' | 'ä' | 'à' | 'â' | 'ã' | 'å' | 'Á' | 'Ä' | 'À' | 'Â' => "a",
            'č' | 'ç' | 'Č' | 'Ç' => "c",
            'ď' | 'Ď' => "d",
            'é' | 'ě' | 'è' | 'ê' | 'ë' | 'É' | 'Ě' | 'È' => "e",
            'í' | 'ì' | 'î' | 'ï' | 'Í' => "i",
            'ľ' | 'ĺ' | 'Ľ' | 'Ĺ' => "l",
            'ň' | 'ñ' | 'Ň' | 'Ñ' => "n",
            'ó' | 'ô' | 'ö' | 'ò' | 'õ' | 'Ó' | 'Ô' | 'Ö' => "o",
            'ŕ' | 'ř' | 'Ŕ' | 'Ř' => "r",
            'š' | 'Š' => "s",
            'ť' | 'Ť' => "t",
            'ú' | 'ů' | 'ü' | 'ù' | 'û' | 'Ú' | 'Ů' | 'Ü' => "u",
            'ý' | 'ÿ' | 'Ý' => "y",
            'ž' | 'Ž' => "z",
            'ß' => "ss",
            // "Buyer's name" is "buyers name".
            '\'' | '’' => "",
            '&' => " and ",
            c if c.is_alphanumeric() => {
                out.extend(c.to_lowercase());
                continue;
            }
            _ => " ",
        };
        out.push_str(mapped);
    }
    let words: Vec<&str> = out.split_whitespace().collect();
    // "Your payout", "The total" - the same label.
    let skip = usize::from(matches!(words.first(), Some(&"your") | Some(&"the")) && words.len() > 1);
    words[skip..].join(" ")
}

fn kind_of(label: &str) -> Option<Kind> {
    let folded = fold(label);
    LABELS.iter().find(|(_, names)| names.contains(&folded.as_str())).map(|(k, _)| *k)
}

/// The label as shown: the mail's own words, without decoration.
fn tidy_label(label: &str) -> String {
    let t = label.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’' && c != ')');
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect::<String>().trim().to_string(),
        None => String::new(),
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The first e-mail address in `s`.
fn email_in(s: &str) -> Option<String> {
    s.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '(' | ')' | '[' | ']' | ',' | ';' | '"' | '\''))
        .map(|w| w.trim_matches(|c: char| matches!(c, '.' | ':' | '!' | '?')))
        .find(|w| {
            let Some((local, domain)) = w.split_once('@') else { return false };
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && w.chars().all(|c| c.is_alphanumeric() || "._%+-@".contains(c))
        })
        .map(str::to_string)
}

/// An address nobody sends tickets to: the shop's own robots and desks.
fn is_robot(addr: &str) -> bool {
    let local = addr.split('@').next().unwrap_or("").to_ascii_lowercase();
    [
        "noreply", "no-reply", "no_reply", "donotreply", "do-not-reply", "support", "help", "info", "service",
        "customerservice", "customer-service", "hello", "contact", "privacy", "unsubscribe", "notifications",
        "notification", "mailer-daemon", "news", "newsletter", "marketing",
    ]
    .iter()
    .any(|r| local == *r || local.starts_with(&format!("{r}.")) || local.starts_with(&format!("{r}+")))
}

/// Money: a currency beside a number with cents, or such a number at all.
fn looks_like_money(v: &str) -> bool {
    let has_symbol = v.contains(['€', '$', '£']) || {
        let upper = v.to_ascii_uppercase();
        ["EUR", "USD", "GBP", "CZK", "PLN", "HUF", "CHF", "SEK", "NOK", "DKK"].iter().any(|c| upper.contains(c))
    };
    let digits: Vec<char> = v.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.iter().all(|d| *d == '0') {
        return false; // "€0.00" says nothing
    }
    let cents = v.as_bytes().windows(3).any(|w| (w[0] == b'.' || w[0] == b',') && w[1].is_ascii_digit() && w[2].is_ascii_digit());
    has_symbol || cents
}

/// Whether `value` is a fair value for `kind`, and the value to keep.
fn accept(kind: Kind, value: &str, strict: bool) -> Option<String> {
    let v = value.trim().trim_end_matches(['.', ',', ';']).trim();
    if v.is_empty() || matches!(v.to_ascii_lowercase().as_str(), "-" | "n/a" | "na" | "none" | "--") {
        return None;
    }
    // Without a colon, a value that reads on as a sentence is not one - an
    // address is written in small letters and still is one.
    if strict && v.chars().next().is_some_and(|c| c.is_lowercase()) && !v.contains('@') {
        return None;
    }
    let has_digit = v.chars().any(|c| c.is_ascii_digit());
    let keep = match kind {
        Kind::Email | Kind::SendTo if v.contains('@') => email_in(v).filter(|e| !is_robot(e))?,
        Kind::Email => return None,
        Kind::Money => {
            if !looks_like_money(v) {
                return None;
            }
            v.to_string()
        }
        Kind::Phone => {
            if v.chars().filter(|c| c.is_ascii_digit()).count() < 6 {
                return None;
            }
            v.to_string()
        }
        Kind::When | Kind::Deadline | Kind::Tickets | Kind::Reference if !has_digit => return None,
        Kind::Name | Kind::SendTo | Kind::Event | Kind::Venue => {
            if v.chars().count() > 80 || !v.chars().any(char::is_alphabetic) {
                return None;
            }
            v.to_string()
        }
        _ => v.to_string(),
    };
    Some(clip(&keep, 100))
}

/// "Label: value" with the colon early in the line - not a time ("19:00")
/// and not a web address ("https://...").
fn split_label(line: &str) -> Option<(&str, &str)> {
    let at = line.find(':')?;
    let (label, rest) = (line[..at].trim(), line[at + 1..].trim());
    if label.is_empty() || label.chars().count() > 40 || label.split_whitespace().count() > 5 {
        return None;
    }
    if label.chars().all(|c| c.is_ascii_digit()) || rest.starts_with("//") {
        return None;
    }
    Some((label, rest))
}

/// A table row without a colon - "Event Oasis Live 25" - read only when its
/// first one to four words are a label TIQR knows.
fn split_known_prefix(line: &str) -> Option<(String, String)> {
    if line.chars().count() > 90 {
        return None;
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    for n in (1..=4.min(words.len().saturating_sub(1))).rev() {
        let label = words[..n].join(" ");
        if kind_of(&label).is_some() {
            return Some((label, words[n..].join(" ")));
        }
    }
    None
}

/// The few facts worth knowing, from an email's text. `own` is the mailbox's
/// own address and `sender` the address the mail came from; neither is ever
/// "where to send".
pub(crate) fn key_facts(text: &str, sender: &str, own: Option<&str>) -> Vec<MailFact> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        // Quoted mail is an older message, not this one.
        .take_while(|l| !l.starts_with("-----Original Message") && !(l.starts_with("On ") && l.ends_with("wrote:")))
        .filter(|l| !l.starts_with('>'))
        .collect();
    let sender = sender.trim().to_ascii_lowercase();
    let own = own.map(|o| o.trim().to_ascii_lowercase());
    let mine = |v: &str| {
        let v = v.to_ascii_lowercase();
        v == sender || own.as_deref() == Some(v.as_str())
    };

    let mut found: Vec<(Kind, MailFact)> = Vec::new();
    let push = |kind: Kind, label: &str, value: String, found: &mut Vec<(Kind, MailFact)>| {
        if found.iter().filter(|(k, _)| *k == kind).count() >= kind.most()
            || found.iter().any(|(_, f)| f.value.eq_ignore_ascii_case(&value))
            || (value.contains('@') && mine(&value))
        {
            return;
        }
        let label = tidy_label(label);
        if label.is_empty() {
            return;
        }
        let private = kind.private() || value.contains('@');
        found.push((kind, MailFact { label, value, private, kind: kind.as_str() }));
    };

    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        if let Some((label, rest)) = split_label(line) {
            if let Some(kind) = kind_of(label) {
                // "Total Proceeds:" with the value on the next line.
                let value = if rest.is_empty() {
                    lines[i + 1..].iter().find(|l| !l.is_empty()).filter(|l| split_label(l).is_none()).copied().unwrap_or("")
                } else {
                    rest
                };
                if let Some(v) = accept(kind, value, false) {
                    push(kind, label, v, &mut found);
                }
                continue;
            }
        }
        if let Some((label, rest)) = split_known_prefix(line) {
            if let Some(kind) = kind_of(&label) {
                if let Some(v) = accept(kind, &rest, true) {
                    push(kind, &label, v, &mut found);
                }
            }
        }
    }

    // "Please transfer the tickets to anna@example.com" - where to send,
    // asked in a sentence rather than beside a label.
    if !found.iter().any(|(k, _)| matches!(k, Kind::SendTo | Kind::Email)) {
        for (i, line) in lines.iter().enumerate() {
            let here = format!("{} {}", line, lines.get(i + 1).copied().unwrap_or(""));
            let low = fold(&here);
            let asks = ["send", "transfer", "deliver", "forward", "assign", "poslat", "posli", "preposlat", "odoslat", "senden", "ubertragen", "enviar", "transferir"]
                .iter()
                .any(|w| low.split(' ').any(|x| x == *w || x.starts_with(w)));
            let tickets = ["ticket", "tickets", "listok", "listky", "vstupenk", "karte", "entrada", "biglietti"]
                .iter()
                .any(|w| low.contains(w));
            if asks && tickets {
                if let Some(addr) = email_in(&here).filter(|a| !is_robot(a) && !mine(a)) {
                    push(Kind::SendTo, "Send to", addr, &mut found);
                    break;
                }
            }
        }
    }

    found.sort_by_key(|(k, _)| *k);
    found.into_iter().take(8).map(|(_, f)| f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(f: &[MailFact]) -> Vec<String> {
        f.iter().map(|x| format!("{}={}{}", x.label, x.value, if x.private { " (private)" } else { "" })).collect()
    }

    #[test]
    fn a_send_your_tickets_mail_gives_where_to_whom_and_how_much() {
        let text = "Hi Marko,\n\nGreat news - your tickets have sold!\n\nPlease send your tickets\n\
                    Buyer's name: Anna Kovacova\nBuyer email: anna.k@example.com\nSend by: 14 Nov 2026, 18:00\n\n\
                    Event: ESPAÑA - INGLATERRA\nDate: 15 Nov 2026\nVenue: Estadio Metropolitano\nQuantity: 2\n\
                    Section: 112\nRow: F\nSeats: 14, 15\nSale ID: 88412093\nSale Price: €180.00\nTotal Proceeds: €162.50\n\n\
                    Questions? Write to help@viagogo.com";
        let f = key_facts(text, "noreply@viagogo.com", Some("biznismarko9@gmail.com"));
        let l = labels(&f);
        assert_eq!(f.len(), 8, "{l:?}");
        assert_eq!(l[0], "Buyer's name=Anna Kovacova (private)");
        assert_eq!(l[1], "Buyer email=anna.k@example.com (private)");
        assert_eq!(l[2], "Send by=14 Nov 2026, 18:00");
        assert!(l.contains(&"Sale Price=€180.00".to_string()) && l.contains(&"Total Proceeds=€162.50".to_string()), "{l:?}");
        assert!(!l.iter().any(|x| x.contains("help@viagogo.com")), "the shop's own desk is not where tickets go");
    }

    #[test]
    fn a_label_with_its_value_on_the_next_line_is_read() {
        let f = key_facts("Shipping Refund:\n€0.00\nTotal Proceeds:\n€720.78\n", "noreply@viagogo.com", None);
        assert_eq!(labels(&f), vec!["Total Proceeds=€720.78"], "a zero amount says nothing");
    }

    #[test]
    fn a_table_row_without_a_colon_is_read_only_when_it_is_clearly_a_value() {
        let text = "Event Oasis Live 25 - Wembley Stadium\nDate of your order is shown below\nTotal paid 412,50 EUR (fees included)\nTotal 2 tickets";
        let l = labels(&key_facts(text, "x@y.com", None));
        assert!(l.contains(&"Event=Oasis Live 25 - Wembley Stadium".to_string()), "{l:?}");
        assert!(l.contains(&"Total paid=412,50 EUR (fees included)".to_string()), "{l:?}");
        assert!(!l.iter().any(|x| x.starts_with("Date")), "prose is not a value: {l:?}");
        assert!(!l.iter().any(|x| x.contains("2 tickets")), "a count is not money: {l:?}");
    }

    #[test]
    fn where_to_send_is_also_found_in_a_sentence() {
        let f = key_facts(
            "Your listing sold.\nPlease transfer the tickets to marek.novak@example.sk as soon as possible.\nThanks, StubHub",
            "noreply@stubhub.com",
            None,
        );
        assert_eq!(labels(&f), vec!["Send to=marek.novak@example.sk (private)"]);
        let none = key_facts("Contact support@stubhub.com about your tickets.", "noreply@stubhub.com", None);
        assert!(none.is_empty(), "{:?}", labels(&none));
    }

    #[test]
    fn the_mailbox_and_the_sender_are_never_where_to_send() {
        let text = "Email: biznismarko9@gmail.com\nRecipient: orders@viagogo.com\nName: Marko";
        let l = labels(&key_facts(text, "orders@viagogo.com", Some("biznismarko9@gmail.com")));
        assert_eq!(l, vec!["Name=Marko (private)"]);
    }

    #[test]
    fn slovak_german_and_spanish_labels_are_read() {
        let l = labels(&key_facts(
            "Meno: Ján Novák\nCelkom: 89,90 €\nPodujatie: Elán - Košice\nDátum: 12.11.2026\nBestellnummer: DE-4412\nTotal a pagar: 120,00 EUR",
            "x@y.sk",
            None,
        ));
        assert!(l.contains(&"Meno=Ján Novák (private)".to_string()), "{l:?}");
        assert!(l.contains(&"Celkom=89,90 €".to_string()), "{l:?}");
        assert!(l.contains(&"Podujatie=Elán - Košice".to_string()), "{l:?}");
        assert!(l.contains(&"Dátum=12.11.2026".to_string()), "{l:?}");
        assert!(l.contains(&"Bestellnummer=DE-4412".to_string()), "{l:?}");
    }

    #[test]
    fn quoted_mail_times_and_web_addresses_are_not_labels() {
        let text = "Doors 19:00\nSee https://example.com/a for more\nOn Mon, 3 Nov 2026, Viagogo wrote:\nTotal: €999.00";
        assert!(key_facts(text, "x@y.com", None).is_empty());
        assert!(key_facts("> Total: €12.00\n", "x@y.com", None).is_empty());
    }

    #[test]
    fn a_long_mail_gives_at_most_eight_lines() {
        let text = (1..40).map(|i| format!("Total: €{i}.00\nSection: {i}\n")).collect::<String>();
        assert!(key_facts(&text, "x@y.com", None).len() <= 8);
    }

    /// The designed mail most marketplaces send: a table, labels in one cell
    /// and values in the next, sometimes without a colon.
    #[test]
    fn a_designed_html_mail_is_read_through_its_text() {
        let html = "<html><body><h1>viagogo</h1><p>Great news - your tickets have sold! Please send your tickets.</p>\
                    <table><tr><td>Buyer&#39;s name:</td><td>Anna Kovacova</td></tr>\
                    <tr><td>Buyer email</td><td><a href=\"mailto:anna.k@example.com\">anna.k@example.com</a></td></tr>\
                    <tr><td>Total Proceeds</td><td><b>&euro;162.50</b></td></tr>\
                    <tr><td>Event</td><td>ESPA&Ntilde;A - INGLATERRA</td></tr></table>\
                    <p><a href=\"https://x.example/r\">Download receipt</a></p></body></html>";
        let text = crate::commands::mail::strip_html(html);
        let l = labels(&key_facts(&text, "noreply@viagogo.com", None));
        assert!(l.contains(&"Buyer's name=Anna Kovacova (private)".to_string()), "{l:?} from {text:?}");
        assert!(l.contains(&"Buyer email=anna.k@example.com (private)".to_string()), "{l:?}");
        assert!(l.contains(&"Total Proceeds=€162.50".to_string()), "{l:?}");
        assert!(l.contains(&"Event=ESPAÑA - INGLATERRA".to_string()), "{l:?}");
    }

    #[test]
    fn labels_fold_accents_and_your() {
        assert_eq!(fold("Your Payout:"), "payout");
        assert_eq!(fold("Číslo objednávky"), "cislo objednavky");
        assert_eq!(fold("Date & Time"), "date and time");
        assert_eq!(kind_of("E-mail"), Some(Kind::Email));
    }
}
