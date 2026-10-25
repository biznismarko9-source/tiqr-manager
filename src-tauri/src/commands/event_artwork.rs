//! 2.116.0 - a picture for every event.
//!
//! The Events page is where a picture is decided; every other screen
//! (Inventory, Orders, Sales) shows the picture of the event a record
//! belongs to and never looks for one of its own.
//!
//! ## What a picture belongs to
//!
//! Three things, kept apart on purpose:
//!
//! * the **event** - one night, one row in `events`;
//! * the **subject** (`Subject::key`, `name:<folded name>`) - what the picture
//!   is OF. Every event with the same name is the same subject, so five dates
//!   of one artist show one picture, and a choice made for one is made for
//!   all five. The key is the whole name, folded (case, accents, punctuation)
//!   but never fuzzy: "Coldplay" and "Coldplay Tribute Band" are two subjects,
//!   and a match keeps both sides in its name, so "Real Madrid vs Barcelona"
//!   and "Real Madrid vs Bayern" are two subjects too - one team's matches are
//!   never assumed to be one event;
//! * the **search** (`Subject::query_key`, `artist:coldplay`) - what is asked
//!   of the picture services. "Coldplay - Prague" and "Coldplay - Vienna" are
//!   two subjects but one search, so the services are asked once.
//!
//! A picture meant for one event only is stored under `event:<event uid>`.
//!
//! ## Who decides, and what can never happen
//!
//! A choice made by hand (`event_artworks`, migration 047) always wins. The
//! services' answers live in a separate table (`artwork_lookups`) that nothing
//! automatic ever copies into the first one and that never leaves this
//! computer - so a search that finishes late, here or on the other computer,
//! can never replace a picture chosen by hand. What is shown for an event is
//! worked out on every read (`shown_for`), in this order:
//!
//! 1. a choice for this event only (`event:<uid>`): a picture, or "none";
//! 2. a choice for its subject: a picture, or "none";
//! 3. a picture chosen by hand for another subject with the same search
//!    ("Coldplay" chosen, "Coldplay - Vienna" shows it too) - never one meant
//!    for a single event;
//! 4. the services' automatic pick, only when the answer was certain;
//! 5. nothing - a neutral placeholder on screen.
//!
//! ## How sure
//!
//! Every candidate carries a confidence and the reasons for it (`score_*`).
//! A name must match exactly once folded; a near name is never more than
//! low. High (>= 0.85) is shown automatically, medium (>= 0.6) waits for a
//! yes, low is never shown unless picked by hand from the list. The event's
//! category says which service to trust and which descriptions fit ("Spanish
//! football club" for a match, "British rock band" for a concert); without a
//! category nothing is ever more than medium unless the artist is famous.
//!
//! ## The services
//!
//! Two, both free and without a key, so nothing here can stop working
//! because an account lapsed: Deezer's artist search for music, and
//! Wikipedia for teams, competitions, festivals and shows. Every request
//! goes from here, not from the screen, with an honest name (`AGENT`), a
//! timeout, a size limit, its own pace per service and a pause after "too
//! many requests"; pictures come only from the two services' own picture
//! servers, through the same checks as pictures in mail
//! (`mail_view::fetch_picture`). A picture is stored as a small data URL, so
//! it shows offline and cannot disappear from a server later.
use crate::commands::mail_facts::fold_words;
use crate::commands::mail_view::{fetch_picture, sniff_image};
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use base64::Engine;
use chrono::{DateTime, Duration as Span, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{Manager, State};

/// How TIQR introduces itself to the picture services - both ask for a
/// name and a way to reach whoever runs it, and nothing more.
const AGENT: &str = concat!(
    "TIQR-Manager/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/biznismarko9-source/tiqr-manager)"
);

const DEEZER_SEARCH: &str = "https://api.deezer.com/search/artist";
const WIKIPEDIA_API: &str = "https://en.wikipedia.org/w/api.php";
/// Where an answer may come from, and where a picture may come from.
const API_HOSTS: &[&str] = &["api.deezer.com", "en.wikipedia.org"];
const PICTURE_HOSTS: &[&str] = &["upload.wikimedia.org"];
const PICTURE_HOST_SUFFIXES: &[&str] = &[".dzcdn.net"];

/// A picture from a service is a thumbnail; anything bigger is not one.
const MAX_THUMB_BYTES: usize = 512 * 1024;
/// A picture added by hand, after the screen has shrunk it.
const MAX_UPLOAD_BYTES: usize = 700 * 1024;
const MAX_ANSWER_BYTES: usize = 1024 * 1024;
/// How many candidates a search keeps and shows.
const MAX_CANDIDATES: usize = 6;
/// The local answers kept, at most - a cache, not a record.
const MAX_LOOKUPS: i64 = 3000;

/// Per computer: "Find pictures for events automatically". On by default.
pub(crate) const ARTWORK_AUTO_KEY: &str = "event_artwork_auto";

// ---------------------------------------------------------------------------
// Names.
// ---------------------------------------------------------------------------

/// Comparable form: `fold_words` (lower case, accents folded, words only,
/// "&" read as "and"), with "+" read as "and" too - "Florence + The Machine"
/// is "Florence and the Machine". Accents typed as separate marks
/// ("Ce\u{301}line") fold like the ones typed whole, and "İ" - whose lower
/// case is two characters - folds to a plain "i", so folding a folded name
/// changes nothing (a key is checked that way, `valid_identity_key`).
pub(crate) fn norm(s: &str) -> String {
    let plain: String = s
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(|c| if c == 'İ' { 'I' } else { c })
        .collect();
    fold_words(&plain.replace('+', " and "))
}

/// Words that say nothing about who is on stage.
const NOISE_WORDS: &[&str] = &[
    "tickets", "ticket", "vstupenky", "vstupenka", "listky", "listok", "entradas", "billets", "karten",
];

fn is_year(word: &str) -> bool {
    let w = word.trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | '\'' | '"' | '!'));
    let year = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) && matches!(&s[..2], "19" | "20");
    let short = |s: &str| s.len() == 2 && s.chars().all(|c| c.is_ascii_digit());
    match w.split_once('/') {
        Some((a, b)) => (year(a) || short(a)) && (year(b) || short(b)),
        None => year(w),
    }
}

/// The name without what changes from night to night or says nothing about
/// the subject: anything in brackets, a year or a season, "tickets". A name
/// that is nothing else ("The 1975") stays as it was.
pub(crate) fn clean(s: &str) -> String {
    let mut bare = String::with_capacity(s.len());
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => bare.push(c),
            _ => {}
        }
    }
    let kept: Vec<&str> =
        bare.split_whitespace().filter(|w| !is_year(w) && !NOISE_WORDS.contains(&norm(w).as_str())).collect();
    let joined = kept.join(" ");
    let out = joined
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | ':' | '|' | ',' | '@' | '/' | '·'))
        .to_string();
    let meaningful = norm(&out);
    if meaningful.is_empty() || meaningful == "the" {
        let fallback = s.split_whitespace().collect::<Vec<_>>().join(" ");
        return fallback.trim().to_string();
    }
    out
}

/// The positions of every separator in `s` (byte offset, length), earliest
/// first and never overlapping. Searched in an ASCII-lowered copy, which has
/// exactly the original's byte offsets - so slicing `s` with them can never
/// land inside a character, whatever language the name is in.
fn separators(s: &str, seps: &[&str]) -> Vec<(usize, usize)> {
    let lower = s.to_ascii_lowercase();
    let mut found: Vec<(usize, usize)> = Vec::new();
    for sep in seps {
        let mut from = 0;
        while let Some(i) = lower[from..].find(sep) {
            found.push((from + i, sep.len()));
            from += i + sep.len();
        }
    }
    found.sort();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, len) in found {
        if out.last().is_some_and(|(j, l)| i < j + l) {
            continue;
        }
        out.push((i, len));
    }
    out
}

const VERSUS: &[&str] = &[" vs. ", " vs ", " versus "];
/// Only where the category says it is sport: " v " is also Slovak for "in"
/// ("Koncert v Bratislave"), and " - " is how a tour is named.
const VERSUS_SPORT: &[&str] = &[" v. ", " v ", " - ", " – ", " — "];

/// "Real Madrid vs Barcelona" -> ("Real Madrid", "Barcelona").
fn split_versus(s: &str, sport: bool) -> Option<(String, String)> {
    let mut seps: Vec<&str> = VERSUS.to_vec();
    if sport {
        seps.extend_from_slice(VERSUS_SPORT);
    }
    let (i, len) = *separators(s, &seps).first()?;
    let (home, away) = (s[..i].trim(), s[i + len..].trim());
    (!home.is_empty() && !away.is_empty()).then(|| (home.to_string(), away.to_string()))
}

const SUBTITLE: &[&str] =
    &[" - ", " – ", " — ", " | ", ": ", " @ ", " feat. ", " feat ", " ft. ", " featuring ", " w/ "];
/// A part of a name that is only a label for what follows ("Concert: X").
const GENERIC: &[&str] = &["concert", "koncert", "live", "show", "tour", "world tour", "event", "the"];

/// "Coldplay - Music of the Spheres" -> "Coldplay". The first part that is
/// not a bare label.
fn subject_part(s: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut at = 0;
    for (i, len) in separators(s, SUBTITLE) {
        parts.push(&s[at..i]);
        at = i + len;
    }
    parts.push(&s[at..]);
    parts
        .into_iter()
        .map(str::trim)
        .find(|p| !p.is_empty() && !GENERIC.contains(&norm(p).as_str()))
        .unwrap_or_else(|| s.trim())
        .to_string()
}

const TRAILING: &[&str] = &[
    "world tour", "stadium tour", "arena tour", "european tour", "europe tour", "farewell tour", "the tour",
    "tour", "in concert", "live", "concert", "koncert", "show",
];

/// "Imagine Dragons World Tour" -> "Imagine Dragons" - never down to nothing.
fn without_trailing(s: &str) -> String {
    let mut words: Vec<&str> = s.split_whitespace().collect();
    loop {
        let folded: Vec<String> = words.iter().map(|w| norm(w)).collect();
        let cut = TRAILING.iter().find_map(|t| {
            let tw: Vec<&str> = t.split(' ').collect();
            (folded.len() > tw.len() && folded[folded.len() - tw.len()..].iter().zip(&tw).all(|(a, b)| a == b))
                .then_some(tw.len())
        });
        match cut {
            Some(n) => words.truncate(words.len() - n),
            None => break,
        }
    }
    words.join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Music,
    Comedy,
    Sports,
    Motorsport,
    Festival,
    Theatre,
    Unknown,
}

fn has_word(folded: &str, word: &str) -> bool {
    format!(" {folded} ").contains(&format!(" {word} "))
}

/// What kind of event this is - from its category first (the names marko
/// gave them, in English or Slovak), from the name only when there is none.
/// With it, for sport, the sport when the category names one.
fn class_of(category: Option<&str>, name: &str) -> (Class, Option<&'static str>) {
    let sport_of = |f: &str| -> Option<&'static str> {
        if ["football", "futbal", "fotbal", "soccer", "fussball", "futbol", "calcio"].iter().any(|k| f.contains(k)) {
            Some("football")
        } else if ["hockey", "hokej"].iter().any(|k| f.contains(k)) || has_word(f, "nhl") {
            Some("ice hockey")
        } else if f.contains("basketbal") || has_word(f, "nba") {
            Some("basketball")
        } else {
            None
        }
    };
    if let Some(cat) = category.map(norm).filter(|c| !c.is_empty()) {
        let f = cat.as_str();
        let any = |ks: &[&str]| ks.iter().any(|k| f.contains(k));
        let word = |ks: &[&str]| ks.iter().any(|k| has_word(f, k));
        if any(&["motor", "formula", "grand prix", "racing", "rally", "nascar"]) || word(&["f1", "motogp"]) {
            return (Class::Motorsport, None);
        }
        if any(&[
            "sport", "football", "futbal", "fotbal", "soccer", "fussball", "futbol", "calcio", "hockey", "hokej",
            "basketbal", "tennis", "tenis", "rugby", "baseball", "volleyball", "volejbal", "handball", "hadzana",
            "boxing", "cricket", "athletic", "atletik",
        ]) || word(&["nba", "nhl", "nfl", "mlb", "ufc", "mma", "box", "darts", "golf", "liga", "league", "match", "zapas"])
        {
            return (Class::Sports, sport_of(f));
        }
        if any(&["comedy", "komedi", "stand up", "standup", "humor"]) {
            return (Class::Comedy, None);
        }
        if any(&["festival"]) || word(&["fest"]) {
            return (Class::Festival, None);
        }
        if any(&["theat", "musical", "muzikal", "divadl", "opera", "ballet", "balet", "circus", "cirkus"]) {
            return (Class::Theatre, None);
        }
        if any(&["concert", "koncert", "music", "hudb", "hip hop", "techno", "metal"])
            || word(&["live", "rap", "rock", "pop", "dj", "edm", "jazz", "tour"])
        {
            return (Class::Music, None);
        }
    }
    let f = norm(name);
    if ["grand prix", "formula 1", "formula1", "moto gp", "motogp", "nascar"].iter().any(|k| f.contains(k)) {
        (Class::Motorsport, None)
    } else if f.contains("festival") {
        (Class::Festival, None)
    } else if ["musical", "divadlo", "theatre", "theater"].iter().any(|k| f.contains(k)) {
        (Class::Theatre, None)
    } else if ["comedy", "stand up"].iter().any(|k| f.contains(k)) {
        (Class::Comedy, None)
    } else if split_versus(name, false).is_some() {
        (Class::Sports, None)
    } else {
        (Class::Unknown, None)
    }
}

/// Which service is asked, and what a description has to say to fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Strategy {
    /// Music and comedy: Deezer, then Wikipedia.
    Artist,
    /// A match (its home side) or a sports subject: Wikipedia.
    Team,
    /// A festival, a show, a race: Wikipedia.
    Show,
    /// No category: both, and never more than medium unless famous.
    Any,
}

impl Strategy {
    fn name(self) -> &'static str {
        match self {
            Strategy::Artist => "artist",
            Strategy::Team => "team",
            Strategy::Show => "show",
            Strategy::Any => "any",
        }
    }
}

/// What a picture is of, worked out from an event the same way every time.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Subject {
    /// `name:<folded name>` - what a choice is stored under.
    pub key: String,
    /// As a person reads it: "Coldplay", "Real Madrid vs Barcelona".
    pub label: String,
    /// What is searched for: "Coldplay", "Real Madrid".
    pub query: String,
    pub strategy: Strategy,
    /// For a match, the sport the category names ("football").
    pub sport: Option<&'static str>,
}

impl Subject {
    pub(crate) fn query_key(&self) -> String {
        format!("{}:{}", self.strategy.name(), norm(&self.query))
    }
}

/// The subject of an event. `artist_team` wins when it is filled in; the
/// name otherwise (it usually is the artist). `None` for a name with
/// nothing in it to search for.
pub(crate) fn subject_for(name: &str, artist_team: Option<&str>, category: Option<&str>) -> Option<Subject> {
    let raw = artist_team.map(str::trim).filter(|s| !s.is_empty()).unwrap_or_else(|| name.trim());
    let cleaned = clean(raw);
    let folded = norm(&cleaned);
    if folded.is_empty() {
        return None;
    }
    let key = format!("name:{}", clip_chars(&folded, 200).trim_end());
    let (class, sport) = class_of(category, raw);
    if let Some((home, away)) = split_versus(&cleaned, class == Class::Sports) {
        let (home, away) = (subject_part(&home), subject_part(&away));
        if !norm(&home).is_empty() && !norm(&away).is_empty() {
            return Some(Subject {
                key,
                label: format!("{home} vs {away}"),
                query: home,
                strategy: Strategy::Team,
                sport,
            });
        }
    }
    let part = subject_part(&cleaned);
    let (strategy, query) = match class {
        Class::Music | Class::Comedy => (Strategy::Artist, without_trailing(&part)),
        Class::Sports => (Strategy::Team, part),
        Class::Motorsport | Class::Festival | Class::Theatre => (Strategy::Show, part),
        Class::Unknown => (Strategy::Any, without_trailing(&part)),
    };
    if norm(&query).is_empty() {
        return None;
    }
    Some(Subject { key, label: query.clone(), query, strategy, sport })
}

fn clip_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// The same name: equal once folded, give or take a leading "The".
fn same_name(a: &str, b: &str) -> bool {
    let strip = |s: &str| s.strip_prefix("the ").unwrap_or(s).to_string();
    !a.is_empty() && strip(a) == strip(b)
}

/// Words a club's name carries around its name: "F.C.", "CF", "1899".
const CLUB_TOKENS: &[&str] = &[
    "fc", "cf", "afc", "sc", "ac", "ss", "ssc", "fk", "sk", "nk", "hc", "hk", "bk", "kk", "cd", "ud", "sd", "rc",
    "rcd", "sv", "vfb", "vfl", "tsg", "bsc", "bv", "cfc", "club", "de", "futbol", "football", "calcio", "the",
];

/// A club's name without its decoration, for comparison only.
fn club_core(folded: &str) -> String {
    let mut tokens: Vec<String> = Vec::new();
    let mut letters = String::new();
    for w in folded.split_whitespace() {
        if w.chars().count() == 1 && w.chars().all(char::is_alphabetic) {
            letters.push_str(w);
            continue;
        }
        if !letters.is_empty() {
            tokens.push(std::mem::take(&mut letters));
        }
        tokens.push(w.to_string());
    }
    if !letters.is_empty() {
        tokens.push(letters);
    }
    tokens
        .into_iter()
        .filter(|t| !CLUB_TOKENS.contains(&t.as_str()) && !t.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Candidates and how sure they are.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Level {
    Low,
    Medium,
    High,
}

pub(crate) const HIGH: f64 = 0.85;
pub(crate) const MEDIUM: f64 = 0.6;

pub(crate) fn level_of(confidence: f64) -> Level {
    if confidence >= HIGH {
        Level::High
    } else if confidence >= MEDIUM {
        Level::Medium
    } else {
        Level::Low
    }
}

/// One picture a service offered, with where it is from and why it is (or
/// is not) believed to be the right one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// `deezer:892`, `wikipedia:Q8682`.
    pub id: String,
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub description: String,
    /// The service's own page about it - the source, for anyone to check.
    pub page: String,
    pub image_url: String,
    /// `cover` for a photo, `contain` for a logo or a crest.
    pub fit: String,
    pub confidence: f64,
    pub level: Level,
    pub reasons: Vec<String>,
    /// The picture itself, when it was fetched. Never stored with the
    /// candidate list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
}

pub(crate) struct DeezerArtist {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub picture: Option<String>,
    pub fans: i64,
}

/// Deezer answers 200 with `{"error": {...}}` when it refuses, and calls an
/// artist without a picture by an address with an empty name in it.
pub(crate) fn parse_deezer(v: &Value) -> Result<Vec<DeezerArtist>, WebError> {
    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
        // 4: "Quota limit exceeded". 800: "no data" is an empty answer.
        return match code {
            4 => Err(WebError::Busy(10)),
            800 => Ok(Vec::new()),
            _ => Err(WebError::Failed(format!("Deezer refused ({code})"))),
        };
    }
    let data = v.get("data").and_then(Value::as_array).ok_or(WebError::Malformed)?;
    Ok(data
        .iter()
        .filter(|a| a.get("type").and_then(Value::as_str).unwrap_or("artist") == "artist")
        .filter_map(|a| {
            let id = a.get("id").and_then(Value::as_i64)?;
            let name = a.get("name").and_then(Value::as_str)?.trim().to_string();
            let picture = a
                .get("picture_medium")
                .and_then(Value::as_str)
                .filter(|p| p.starts_with("https://") && !p.contains("/artist//"))
                .map(str::to_string);
            Some(DeezerArtist {
                id,
                name,
                link: a.get("link").and_then(Value::as_str).unwrap_or_default().to_string(),
                picture,
                fans: a.get("nb_fan").and_then(Value::as_i64).unwrap_or(0).max(0),
            })
        })
        .collect())
}

fn plural(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{} {many}", group_digits(n))
    }
}

fn group_digits(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// Deezer's artists for `query`, each with its confidence.
pub(crate) fn score_deezer(query: &str, strategy: Strategy, artists: &[DeezerArtist]) -> Vec<Candidate> {
    let q = norm(query);
    let pictured: Vec<&DeezerArtist> = artists.iter().filter(|a| a.picture.is_some()).collect();
    let namesakes: Vec<&&DeezerArtist> = pictured.iter().filter(|a| same_name(&norm(&a.name), &q)).collect();
    pictured
        .iter()
        .take(MAX_CANDIDATES)
        .map(|a| {
            let mut why: Vec<String> = Vec::new();
            let mut c: f64;
            if same_name(&norm(&a.name), &q) {
                why.push("the same name".to_string());
                let rival = namesakes.iter().filter(|b| b.id != a.id).map(|b| b.fans).max();
                match rival {
                    None => {
                        c = 0.9;
                        why.push("the only artist on Deezer with this name".to_string());
                    }
                    Some(r) if a.fans >= 10_000 && a.fans >= r.max(1).saturating_mul(20) => {
                        c = 0.88;
                        why.push("far better known than the others with this name".to_string());
                    }
                    Some(r) if a.fans >= r => {
                        c = 0.65;
                        why.push("other artists have the same name".to_string());
                    }
                    Some(_) => {
                        c = 0.5;
                        why.push("a less known artist with the same name".to_string());
                    }
                }
                if a.fans < 1_000 {
                    c = c.min(0.62);
                    why.push("very few fans".to_string());
                }
                if strategy == Strategy::Any && a.fans < 1_000_000 {
                    c = c.min(0.75);
                    why.push("the event has no category to say it is music".to_string());
                }
            } else {
                c = 0.3;
                why.push("a different name".to_string());
            }
            Candidate {
                id: format!("deezer:{}", a.id),
                source: "deezer".to_string(),
                source_id: a.id.to_string(),
                title: a.name.clone(),
                description: format!("{} on Deezer", plural(a.fans, "fan", "fans")),
                page: if a.link.starts_with("https://") {
                    a.link.clone()
                } else {
                    format!("https://www.deezer.com/artist/{}", a.id)
                },
                image_url: a.picture.clone().unwrap_or_default(),
                fit: "cover".to_string(),
                confidence: c,
                level: level_of(c),
                reasons: why,
                thumb: None,
            }
        })
        .collect()
}

pub(crate) struct WikiPage {
    pub index: i64,
    pub page_id: i64,
    pub title: String,
    pub description: String,
    pub thumb: Option<String>,
    pub qid: Option<String>,
    pub disambiguation: bool,
}

/// `action=query&generator=search&prop=pageimages|description|pageprops`,
/// `formatversion=2`: pages as a list, each with its place in the search.
pub(crate) fn parse_wikipedia(v: &Value) -> Result<Vec<WikiPage>, WebError> {
    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(Value::as_str).unwrap_or_default();
        return if code == "ratelimited" || code == "maxlag" {
            Err(WebError::Busy(30))
        } else {
            Err(WebError::Failed(format!("Wikipedia refused ({})", clip_chars(code, 40))))
        };
    }
    let Some(query) = v.get("query") else {
        // A search that found nothing has no `query` at all.
        return if v.is_object() { Ok(Vec::new()) } else { Err(WebError::Malformed) };
    };
    let pages = query.get("pages").and_then(Value::as_array).ok_or(WebError::Malformed)?;
    let mut out: Vec<WikiPage> = pages
        .iter()
        .filter_map(|p| {
            let title = p.get("title").and_then(Value::as_str)?.to_string();
            let props = p.get("pageprops");
            Some(WikiPage {
                index: p.get("index").and_then(Value::as_i64).unwrap_or(99),
                page_id: p.get("pageid").and_then(Value::as_i64).unwrap_or(0),
                title,
                description: p.get("description").and_then(Value::as_str).unwrap_or_default().to_string(),
                thumb: p
                    .get("thumbnail")
                    .and_then(|t| t.get("source"))
                    .and_then(Value::as_str)
                    .filter(|s| s.starts_with("https://"))
                    .map(str::to_string),
                qid: props.and_then(|pp| pp.get("wikibase_item")).and_then(Value::as_str).map(str::to_string),
                disambiguation: props.is_some_and(|pp| pp.get("disambiguation").is_some()),
            })
        })
        .collect();
    out.sort_by_key(|p| p.index);
    Ok(out)
}

const ARTIST_WORDS: &[&str] = &[
    "band", "singer", "rapper", "musician", "songwriter", "dj", "disc jockey", "duo", "trio", "group", "comedian",
    "composer", "producer", "orchestra", "choir", "vocalist", "guitarist", "pianist", "musical group", "music group",
    "rock", "pop", "hip hop", "boy band", "girl group", "entertainer", "stand up",
];
const TEAM_WORDS: &[&str] = &[
    "football club", "soccer club", "football team", "soccer team", "hockey team", "hockey club",
    "basketball team", "basketball club", "national team", "sports club", "club", "team", "franchise",
    "tournament", "championship", "competition", "cup", "league", "grand slam", "final",
];
const SHOW_WORDS: &[&str] = &[
    "festival", "musical", "opera", "ballet", "play", "theatre", "theater", "show", "tour", "grand prix", "race",
    "motor racing", "tournament", "championship", "competition", "cup", "circus", "exhibition", "event",
];

fn words_for(strategy: Strategy) -> Vec<&'static str> {
    match strategy {
        Strategy::Artist => ARTIST_WORDS.to_vec(),
        Strategy::Team => TEAM_WORDS.to_vec(),
        Strategy::Show => SHOW_WORDS.to_vec(),
        Strategy::Any => [ARTIST_WORDS, TEAM_WORDS, SHOW_WORDS].concat(),
    }
}

/// "Muse (band)" -> ("Muse", "band").
fn title_parts(title: &str) -> (String, String) {
    match title.rfind(" (") {
        Some(i) if title.ends_with(')') => (title[..i].to_string(), title[i + 2..title.len() - 1].to_string()),
        _ => (title.to_string(), String::new()),
    }
}

/// Wikipedia's pages for `query`, each with its confidence.
pub(crate) fn score_wikipedia(query: &str, strategy: Strategy, pages: &[WikiPage]) -> Vec<Candidate> {
    let q = norm(query);
    let words = words_for(strategy);
    let usable: Vec<&WikiPage> = pages.iter().filter(|p| !p.disambiguation && p.thumb.is_some()).collect();
    let judged: Vec<(&WikiPage, bool, bool)> = usable
        .iter()
        .map(|p| {
            let (base, qualifier) = title_parts(&p.title);
            let b = norm(&base);
            let name_ok = same_name(&b, &q)
                || (strategy != Strategy::Artist && !club_core(&q).is_empty() && club_core(&b) == club_core(&q))
                || (strategy == Strategy::Team
                    && (b.starts_with(&format!("{q} national ")) || b.starts_with(&format!("{q} men s national "))));
            let about = norm(&format!("{} {}", p.description, qualifier));
            let kind_ok = words.iter().any(|w| has_word(&about, w));
            (*p, name_ok, kind_ok)
        })
        .collect();
    let fitting = judged.iter().filter(|(_, n, k)| *n && *k).count();
    judged
        .iter()
        .take(MAX_CANDIDATES)
        .enumerate()
        .map(|(rank, (p, name_ok, kind_ok))| {
            let mut why: Vec<String> = Vec::new();
            let mut c: f64 = match (name_ok, kind_ok) {
                (true, true) => 0.9,
                (true, false) => 0.45,
                (false, true) => 0.4,
                (false, false) => 0.15,
            };
            why.push(if *name_ok { "the same name" } else { "a different name" }.to_string());
            if !p.description.is_empty() {
                why.push(format!("Wikipedia: \"{}\"", clip_chars(&p.description, 80)));
            }
            if !kind_ok {
                why.push(
                    match strategy {
                        Strategy::Artist => "not described as an artist",
                        Strategy::Team => "not described as a team or a competition",
                        Strategy::Show => "not described as a show or an event",
                        Strategy::Any => "not described as anything an event is about",
                    }
                    .to_string(),
                );
            }
            // A page above it that also fits (another club, another show) is
            // a real rival; a city of the same name above a club is not.
            if c >= MEDIUM && judged[..rank].iter().any(|(_, _, k)| *k) {
                c -= 0.12;
                why.push("Wikipedia ranks another fitting page first".to_string());
            }
            if c >= HIGH && fitting > 1 {
                c = 0.7;
                why.push("more than one page fits".to_string());
            }
            if strategy == Strategy::Any && c > 0.75 {
                c = 0.75;
                why.push("the event has no category to confirm it".to_string());
            }
            let thumb_url = p.thumb.clone().unwrap_or_default();
            let lower = thumb_url.to_ascii_lowercase();
            let fit = if lower.contains(".svg") || lower.ends_with(".png") { "contain" } else { "cover" };
            let (source_id, id) = match &p.qid {
                Some(q) => (q.clone(), format!("wikipedia:{q}")),
                None => (p.page_id.to_string(), format!("wikipedia:page{}", p.page_id)),
            };
            Candidate {
                id,
                source: "wikipedia".to_string(),
                source_id,
                title: p.title.clone(),
                description: p.description.clone(),
                page: format!("https://en.wikipedia.org/?curid={}", p.page_id),
                image_url: thumb_url,
                fit: fit.to_string(),
                confidence: c,
                level: level_of(c),
                reasons: why,
                thumb: None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The network.
// ---------------------------------------------------------------------------

/// Why a service could not be asked - each one a different thing to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WebError {
    /// No connection at all.
    Offline,
    TimedOut,
    /// "Too many requests" - wait this many seconds.
    Busy(u64),
    /// Answered with this status.
    Refused(u16),
    /// Answered with something that is not what it should be.
    Malformed,
    /// An address that is not one of the services'.
    NotAllowed,
    Failed(String),
}

impl WebError {
    /// For a person: what happened, in a sentence, without addresses or keys.
    pub(crate) fn message(&self) -> String {
        match self {
            WebError::Offline => "No internet connection - the picture is looked for again later.".to_string(),
            WebError::TimedOut => "The picture service did not answer in time.".to_string(),
            WebError::Busy(_) => "The picture service asked to slow down - trying again in a while.".to_string(),
            WebError::Refused(code) => format!("The picture service refused the request ({code})."),
            WebError::Malformed => "The picture service sent an answer TIQR could not read.".to_string(),
            WebError::NotAllowed => "That picture is not from a picture service TIQR trusts.".to_string(),
            WebError::Failed(why) => format!("The picture could not be fetched ({}).", clip_chars(why, 80)),
        }
    }

    fn is_offline(&self) -> bool {
        matches!(self, WebError::Offline)
    }
}

pub(crate) trait Web: Sync {
    fn json(&self, url: &str) -> Result<Value, WebError>;
    fn picture(&self, url: &str) -> Result<Vec<u8>, WebError>;
    /// Whose pace, pause and pictures these are. The real services share one
    /// (`""`); each test's fake has its own, so tests running side by side
    /// never slow, pause or answer each other.
    fn scope(&self) -> &str {
        ""
    }
}

fn host_of(url: &str) -> Option<String> {
    let u = reqwest::Url::parse(url).ok()?;
    (u.scheme() == "https").then(|| u.host_str().map(str::to_ascii_lowercase)).flatten()
}

pub(crate) fn picture_host_allowed(url: &str) -> bool {
    host_of(url).is_some_and(|h| {
        PICTURE_HOSTS.contains(&h.as_str()) || PICTURE_HOST_SUFFIXES.iter().any(|s| h.ends_with(s))
    })
}

struct LiveWeb {
    client: Option<reqwest::blocking::Client>,
}

impl LiveWeb {
    fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(AGENT)
            .build()
            .ok();
        LiveWeb { client }
    }
}

impl Web for LiveWeb {
    fn json(&self, url: &str) -> Result<Value, WebError> {
        let host = host_of(url).ok_or(WebError::NotAllowed)?;
        if !API_HOSTS.contains(&host.as_str()) {
            return Err(WebError::NotAllowed);
        }
        let client = self.client.as_ref().ok_or_else(|| WebError::Failed("no web client".to_string()))?;
        let resp = client.get(url).header(reqwest::header::ACCEPT, "application/json").send().map_err(|e| {
            if e.is_timeout() {
                WebError::TimedOut
            } else if e.is_connect() {
                WebError::Offline
            } else {
                WebError::Failed("the request failed".to_string())
            }
        })?;
        let status = resp.status();
        if status.as_u16() == 429 {
            let wait = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(60);
            return Err(WebError::Busy(wait.clamp(5, 3600)));
        }
        if status.is_server_error() {
            return Err(WebError::Failed(format!("answered {}", status.as_u16())));
        }
        if !status.is_success() {
            return Err(WebError::Refused(status.as_u16()));
        }
        let mut body = Vec::new();
        resp.take(MAX_ANSWER_BYTES as u64 + 1).read_to_end(&mut body).map_err(|_| WebError::TimedOut)?;
        if body.len() > MAX_ANSWER_BYTES {
            return Err(WebError::Malformed);
        }
        serde_json::from_slice(&body).map_err(|_| WebError::Malformed)
    }

    fn picture(&self, url: &str) -> Result<Vec<u8>, WebError> {
        if !picture_host_allowed(url) {
            return Err(WebError::NotAllowed);
        }
        fetch_picture(url, AGENT, MAX_THUMB_BYTES).map_err(|why| match why.as_str() {
            "no answer" | "unknown host" => WebError::Offline,
            other => WebError::Failed(other.to_string()),
        })
    }
}

/// Each service in its own time: a short gap between two requests to it, and
/// a pause after it said "too many requests".
struct Pace {
    next: Instant,
    blocked_until: Option<Instant>,
}

fn paces() -> &'static Mutex<HashMap<String, Pace>> {
    static P: OnceLock<Mutex<HashMap<String, Pace>>> = OnceLock::new();
    P.get_or_init(|| Mutex::new(HashMap::new()))
}

fn take_turn(web: &dyn Web, service: &str, gap: Duration) -> Result<(), WebError> {
    let gap = if cfg!(test) { Duration::ZERO } else { gap };
    let wait = {
        let mut all = paces().lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let p = all.entry(format!("{}|{service}", web.scope())).or_insert(Pace { next: now, blocked_until: None });
        if let Some(until) = p.blocked_until {
            if until > now {
                return Err(WebError::Busy((until - now).as_secs().max(1)));
            }
            p.blocked_until = None;
        }
        let wait = p.next.saturating_duration_since(now);
        p.next = now + wait + gap;
        wait
    };
    if !wait.is_zero() {
        std::thread::sleep(wait);
    }
    Ok(())
}

fn note_busy(web: &dyn Web, service: &str, err: &WebError) {
    if let WebError::Busy(secs) = err {
        let mut all = paces().lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let p = all.entry(format!("{}|{service}", web.scope())).or_insert(Pace { next: now, blocked_until: None });
        p.blocked_until = Some(now + Duration::from_secs(*secs));
    }
}

/// After "no connection", automatic searches rest for a while - a computer
/// without internet should not try every event, one by one, every time.
fn offline_until() -> &'static Mutex<HashMap<String, Instant>> {
    static O: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    O.get_or_init(|| Mutex::new(HashMap::new()))
}

const OFFLINE_PAUSE: Duration = Duration::from_secs(180);

fn resting(web: &dyn Web) -> bool {
    offline_until().lock().unwrap_or_else(|e| e.into_inner()).get(web.scope()).is_some_and(|t| *t > Instant::now())
}

fn note_result<T>(web: &dyn Web, r: &Result<T, WebError>) {
    let mut o = offline_until().lock().unwrap_or_else(|e| e.into_inner());
    match r {
        Err(e) if e.is_offline() => {
            o.insert(web.scope().to_string(), Instant::now() + OFFLINE_PAUSE);
        }
        Ok(_) => {
            o.remove(web.scope());
        }
        Err(_) => {}
    }
}

fn encode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn ask_deezer(web: &dyn Web, query: &str, strategy: Strategy) -> Result<Vec<Candidate>, WebError> {
    take_turn(web, "deezer", Duration::from_millis(250))?;
    let url = format!("{DEEZER_SEARCH}?q={}&limit=10", encode(query));
    let answer = web.json(&url).and_then(|v| parse_deezer(&v));
    if let Err(e) = &answer {
        note_busy(web, "deezer", e);
    }
    Ok(score_deezer(query, strategy, &answer?))
}

fn ask_wikipedia(web: &dyn Web, search: &str, query: &str, strategy: Strategy) -> Result<Vec<Candidate>, WebError> {
    take_turn(web, "wikipedia", Duration::from_millis(200))?;
    let url = format!(
        "{WIKIPEDIA_API}?action=query&format=json&formatversion=2&generator=search&gsrsearch={}&gsrlimit=6\
         &prop=pageimages%7Cdescription%7Cpageprops&piprop=thumbnail&pithumbsize=250&pilicense=any\
         &ppprop=wikibase_item%7Cdisambiguation&redirects=1",
        encode(search)
    );
    let answer = web.json(&url).and_then(|v| parse_wikipedia(&v));
    if let Err(e) = &answer {
        note_busy(web, "wikipedia", e);
    }
    Ok(score_wikipedia(query, strategy, &answer?))
}

fn best_level(cands: &[Candidate]) -> Option<Level> {
    cands.iter().map(|c| c.level).max()
}

/// Asks the services for `subject`, in the order its kind calls for, and
/// stops at the first certain answer. A service that cannot be reached is
/// only an error when nothing at all was found.
pub(crate) fn search(web: &dyn Web, subject: &Subject) -> Result<Vec<Candidate>, WebError> {
    let mut found: Vec<Candidate> = Vec::new();
    let mut first_error: Option<WebError> = None;
    let mut keep = |r: Result<Vec<Candidate>, WebError>, found: &mut Vec<Candidate>| match r {
        Ok(v) => found.extend(v),
        Err(e) => {
            first_error.get_or_insert(e);
        }
    };
    let q = subject.query.as_str();
    match subject.strategy {
        Strategy::Artist => {
            keep(ask_deezer(web, q, Strategy::Artist), &mut found);
            if best_level(&found) < Some(Level::High) {
                keep(ask_wikipedia(web, q, q, Strategy::Artist), &mut found);
            }
        }
        Strategy::Team => {
            keep(ask_wikipedia(web, q, q, Strategy::Team), &mut found);
            // "Liverpool" finds the city first; "Liverpool football" finds
            // the club, and "Slovakia football" the national team.
            if best_level(&found) < Some(Level::High) {
                let hint = subject.sport.unwrap_or("club");
                keep(ask_wikipedia(web, &format!("{q} {hint}"), q, Strategy::Team), &mut found);
            }
        }
        Strategy::Show => keep(ask_wikipedia(web, q, q, Strategy::Show), &mut found),
        Strategy::Any => {
            keep(ask_deezer(web, q, Strategy::Any), &mut found);
            if best_level(&found) < Some(Level::High) {
                keep(ask_wikipedia(web, q, q, Strategy::Any), &mut found);
            }
        }
    }
    if found.is_empty() {
        if let Some(e) = first_error {
            return Err(e);
        }
    }
    Ok(rank(found))
}

/// Best first, one per picture, and never two certain answers that disagree:
/// two different pictures both "certain" means neither is.
fn rank(mut found: Vec<Candidate>) -> Vec<Candidate> {
    found.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen: HashSet<String> = HashSet::new();
    found.retain(|c| !c.image_url.is_empty() && seen.insert(c.image_url.clone()));
    let certain: Vec<String> = found.iter().filter(|c| c.level == Level::High).map(|c| c.source.clone()).collect();
    if certain.len() > 1 {
        let sources: HashSet<&String> = certain.iter().collect();
        if sources.len() > 1 {
            for c in found.iter_mut().filter(|c| c.level == Level::High) {
                c.confidence = 0.75;
                c.level = Level::Medium;
                c.reasons.push("another service found a different picture".to_string());
            }
        }
    }
    found.truncate(MAX_CANDIDATES);
    found
}

/// Pictures by address, and the order they came in (the oldest go first).
type KeptThumbs = (HashMap<String, String>, VecDeque<String>);

/// The pictures already fetched this session, by address - so opening the
/// picker twice does not fetch them twice.
fn thumbs() -> &'static Mutex<KeptThumbs> {
    static T: OnceLock<Mutex<KeptThumbs>> = OnceLock::new();
    T.get_or_init(|| Mutex::new((HashMap::new(), VecDeque::new())))
}

const MAX_KEPT_THUMBS: usize = 240;

fn thumb_kept(web: &dyn Web, url: &str) -> Option<String> {
    thumbs().lock().unwrap_or_else(|e| e.into_inner()).0.get(&format!("{}|{url}", web.scope())).cloned()
}

fn keep_thumb(web: &dyn Web, url: &str, src: &str) {
    let key = format!("{}|{url}", web.scope());
    let mut t = thumbs().lock().unwrap_or_else(|e| e.into_inner());
    if t.0.insert(key.clone(), src.to_string()).is_none() {
        t.1.push_back(key);
        while t.1.len() > MAX_KEPT_THUMBS {
            if let Some(old) = t.1.pop_front() {
                t.0.remove(&old);
            }
        }
    }
}

pub(crate) fn data_url(bytes: &[u8]) -> Option<String> {
    let mime = sniff_image(bytes)?;
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// A stored or handed-in picture, checked: a data URL of a picture type, no
/// bigger than `max`, whose bytes really are that picture. Returned in a
/// clean form, its type read from the bytes rather than taken from the label.
pub(crate) fn checked_data_url(s: &str, max: usize) -> Option<String> {
    let rest = s.trim().strip_prefix("data:")?;
    let (mime, b64) = rest.split_once(";base64,")?;
    if !matches!(mime, "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/avif") {
        return None;
    }
    if b64.len() > max / 3 * 4 + 8 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok()?;
    if bytes.is_empty() || bytes.len() > max {
        return None;
    }
    data_url(&bytes)
}

/// Fetches the pictures of `cands` that are not here yet - a few at a time,
/// each through the checks - and drops the candidates whose picture could
/// not be had: one that cannot be shown cannot be offered.
fn with_thumbs(web: &dyn Web, mut cands: Vec<Candidate>, network: bool) -> Vec<Candidate> {
    for c in cands.iter_mut() {
        if c.thumb.is_none() {
            c.thumb = thumb_kept(web, &c.image_url);
        }
    }
    if network {
        let missing: Vec<usize> = (0..cands.len()).filter(|&i| cands[i].thumb.is_none()).collect();
        let fetched: Vec<(usize, Option<String>)> = std::thread::scope(|s| {
            let handles: Vec<_> = missing
                .iter()
                .map(|&i| {
                    let url = cands[i].image_url.clone();
                    s.spawn(move || (i, web.picture(&url).ok().and_then(|b| data_url(&b))))
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        for (i, src) in fetched {
            if let Some(src) = src {
                keep_thumb(web, &cands[i].image_url, &src);
                cands[i].thumb = Some(src);
            }
        }
        cands.retain(|c| c.thumb.is_some());
    }
    cands
}

/// What a search found, with its pictures. Something worth showing whose
/// pictures could not be downloaded is a failure to try again soon, not
/// "nothing found" - that would wait two weeks.
fn pictured(web: &dyn Web, cands: Vec<Candidate>) -> Result<Vec<Candidate>, WebError> {
    let worth = cands.iter().any(|c| c.level >= Level::Medium);
    let out = with_thumbs(web, cands, true);
    if worth && !out.iter().any(|c| c.level >= Level::Medium) {
        return Err(WebError::Failed("the pictures could not be downloaded".to_string()));
    }
    Ok(out)
}

/// One search per subject at a time, across every caller: a second asks
/// for the same thing while the first is out, it waits for that answer
/// instead of sending the same request again.
fn flights() -> &'static (Mutex<HashSet<String>>, Condvar) {
    static F: OnceLock<(Mutex<HashSet<String>>, Condvar)> = OnceLock::new();
    F.get_or_init(|| (Mutex::new(HashSet::new()), Condvar::new()))
}

struct Flight(String);

impl Flight {
    /// `None`: another caller had this search out, and it has now come back
    /// (or `wait` ran out) - read its answer instead.
    fn begin(key: &str, wait: Duration) -> Option<Flight> {
        let (set, done) = flights();
        let mut out = set.lock().unwrap_or_else(|e| e.into_inner());
        if out.insert(key.to_string()) {
            return Some(Flight(key.to_string()));
        }
        let deadline = Instant::now() + wait;
        while out.contains(key) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            out = done.wait_timeout(out, left).unwrap_or_else(|e| e.into_inner()).0;
        }
        None
    }
}

impl Drop for Flight {
    fn drop(&mut self) {
        let (set, done) = flights();
        set.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.0);
        done.notify_all();
    }
}

// ---------------------------------------------------------------------------
// The two tables.
// ---------------------------------------------------------------------------

/// The uid of a subject's row, the same on every computer: derived from its
/// key, never random (see migration 047 for why).
pub(crate) fn artwork_uid(identity_key: &str) -> String {
    let digest = Sha256::digest(format!("event_artworks\u{0}{identity_key}").as_bytes());
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

fn valid_identity_key(key: &str) -> bool {
    if let Some(rest) = key.strip_prefix("name:") {
        !rest.trim().is_empty() && rest.chars().count() <= 200 && norm(rest) == rest
    } else if let Some(uid) = key.strip_prefix("event:") {
        !uid.is_empty() && uid.len() <= 64 && uid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    } else {
        false
    }
}

fn valid_query_key(key: &str) -> bool {
    match key.split_once(':') {
        Some((s, rest)) => {
            matches!(s, "artist" | "team" | "show" | "any") && !rest.is_empty() && rest.chars().count() <= 220
        }
        None => false,
    }
}

fn stamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_stamp(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}

/// A choice made by hand, as it is stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub key: String,
    pub state: String,
    pub label: String,
    pub source: String,
    pub source_title: String,
    pub source_page: String,
    pub updated_at: String,
}

fn decision(conn: &Connection, key: &str) -> AppResult<Option<Decision>> {
    Ok(conn
        .query_row(
            "SELECT identity_key, state, label, source, source_title, source_page, updated_at
               FROM event_artworks WHERE identity_key = ?1",
            [key],
            |r| {
                Ok(Decision {
                    key: r.get(0)?,
                    state: r.get(1)?,
                    label: r.get(2)?,
                    source: r.get(3)?,
                    source_title: r.get(4)?,
                    source_page: r.get(5)?,
                    updated_at: r.get(6)?,
                })
            },
        )
        .optional()?)
}

/// The newest picture chosen by hand for another subject with the same
/// search - never one meant for a single event.
fn earlier_for(conn: &Connection, query_key: &str, except: &str) -> AppResult<Option<Decision>> {
    let key: Option<String> = conn
        .query_row(
            "SELECT identity_key FROM event_artworks
              WHERE query_key = ?1 AND state = 'set' AND image IS NOT NULL
                AND identity_key LIKE 'name:%' AND identity_key <> ?2
              ORDER BY updated_at DESC, id DESC LIMIT 1",
            params![query_key, except],
            |r| r.get(0),
        )
        .optional()?;
    match key {
        Some(k) => decision(conn, &k),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LookupRow {
    outcome: String,
    level: String,
    candidates: Vec<Candidate>,
    pick: Option<Candidate>,
    has_image: bool,
    error: Option<String>,
    attempts: i64,
    checked_at: String,
    retry_after: Option<String>,
}

fn lookup_row(conn: &Connection, query_key: &str) -> AppResult<Option<LookupRow>> {
    Ok(conn
        .query_row(
            "SELECT outcome, level, candidates, pick, image IS NOT NULL, error, attempts, checked_at, retry_after
               FROM artwork_lookups WHERE query_key = ?1",
            [query_key],
            |r| {
                Ok(LookupRow {
                    outcome: r.get(0)?,
                    level: r.get(1)?,
                    // A list that cannot be read is an empty one: it is only
                    // a cache, and the next search writes it again.
                    candidates: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                    pick: r.get::<_, Option<String>>(3)?.and_then(|p| serde_json::from_str(&p).ok()),
                    has_image: r.get(4)?,
                    error: r.get(5)?,
                    attempts: r.get(6)?,
                    checked_at: r.get(7)?,
                    retry_after: r.get(8)?,
                })
            },
        )
        .optional()?)
}

/// How long an answer stands before it is asked again on its own. A certain
/// answer stands until someone asks for another.
fn due(row: Option<&LookupRow>, now: DateTime<Utc>, by_hand: bool) -> bool {
    let Some(row) = row else { return true };
    let checked = parse_stamp(&row.checked_at).unwrap_or(now - Span::days(3650));
    match row.outcome.as_str() {
        "match" => false,
        "ask" => now - checked > Span::days(30),
        "none" => now - checked > Span::days(if by_hand { 1 } else { 14 }),
        _ => {
            // failed: by hand, at once - unless the service asked to wait.
            let waits = row.retry_after.as_deref().and_then(parse_stamp).is_some_and(|t| t > now);
            !waits || (by_hand && !row.error.as_deref().unwrap_or_default().contains("slow down"))
        }
    }
}

fn backoff(attempts: i64) -> Span {
    match attempts {
        ..=1 => Span::minutes(10),
        2 => Span::hours(1),
        3 => Span::hours(6),
        _ => Span::hours(24),
    }
}

/// Writes what a search came back with. A certain answer keeps its picture
/// as the automatic pick; a failure never replaces a certain answer that is
/// already here - it only says when to try again.
fn store_lookup(
    conn: &Connection,
    query_key: &str,
    result: &Result<Vec<Candidate>, WebError>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let at = stamp(now);
    match result {
        Ok(cands) => {
            let shown: Vec<&Candidate> = cands.iter().filter(|c| c.thumb.is_some()).collect();
            let pick = shown.iter().find(|c| c.level == Level::High).copied();
            let (outcome, level) = match (pick, shown.first()) {
                (Some(p), _) => ("match", p.level),
                (None, Some(top)) if top.level >= Level::Medium => ("ask", top.level),
                (None, Some(top)) => ("none", top.level),
                (None, None) => ("none", Level::Low),
            };
            let bare: Vec<Candidate> = cands.iter().map(|c| Candidate { thumb: None, ..c.clone() }).collect();
            let level_name = serde_json::to_value(level).ok().and_then(|v| v.as_str().map(str::to_string));
            conn.execute(
                "INSERT INTO artwork_lookups
                   (query_key, outcome, level, candidates, pick, image, fit, error, attempts, checked_at, retry_after)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 0, ?8, NULL)
                 ON CONFLICT(query_key) DO UPDATE SET
                   outcome = excluded.outcome, level = excluded.level, candidates = excluded.candidates,
                   pick = excluded.pick, image = excluded.image, fit = excluded.fit, error = NULL,
                   attempts = 0, checked_at = excluded.checked_at, retry_after = NULL",
                params![
                    query_key,
                    outcome,
                    level_name.unwrap_or_default(),
                    serde_json::to_string(&bare).unwrap_or_else(|_| "[]".to_string()),
                    pick.map(|p| serde_json::to_string(&Candidate { thumb: None, ..(*p).clone() }).unwrap_or_default()),
                    pick.and_then(|p| p.thumb.clone()),
                    pick.map(|p| p.fit.clone()).unwrap_or_else(|| "cover".to_string()),
                    at,
                ],
            )?;
        }
        Err(e) => {
            let previous = lookup_row(conn, query_key)?;
            let attempts = previous.as_ref().map(|p| p.attempts).unwrap_or(0) + 1;
            let wait = match e {
                WebError::Busy(secs) => Span::seconds(*secs as i64).max(Span::minutes(1)),
                _ => backoff(attempts),
            };
            let retry = stamp(now + wait);
            if previous.as_ref().is_some_and(|p| p.outcome == "match") {
                conn.execute(
                    "UPDATE artwork_lookups SET error = ?2, attempts = ?3, retry_after = ?4 WHERE query_key = ?1",
                    params![query_key, e.message(), attempts, retry],
                )?;
            } else {
                conn.execute(
                    "INSERT INTO artwork_lookups (query_key, outcome, level, candidates, error, attempts, checked_at, retry_after)
                     VALUES (?1, 'failed', '', '[]', ?2, ?3, ?4, ?5)
                     ON CONFLICT(query_key) DO UPDATE SET
                       outcome = 'failed', level = '', error = excluded.error, attempts = excluded.attempts,
                       checked_at = excluded.checked_at, retry_after = excluded.retry_after,
                       pick = NULL, image = NULL",
                    params![query_key, e.message(), attempts, at, retry],
                )?;
            }
        }
    }
    prune_lookups(conn)?;
    Ok(())
}

/// A cache stays a cache: past `MAX_LOOKUPS`, the oldest answers that hold
/// no picture go first.
fn prune_lookups(conn: &Connection) -> AppResult<()> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM artwork_lookups", [], |r| r.get(0))?;
    if n > MAX_LOOKUPS {
        conn.execute(
            "DELETE FROM artwork_lookups WHERE query_key IN (
               SELECT query_key FROM artwork_lookups WHERE outcome <> 'match'
                ORDER BY checked_at LIMIT ?1)",
            [n - MAX_LOOKUPS],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// What every screen shows.
// ---------------------------------------------------------------------------

/// Which picture an event shows: `key` names it for `event_artwork_images`
/// (`d:` a choice made by hand, `a:` an automatic pick), `rev` changes when
/// the picture does.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkRef {
    pub event_id: i64,
    pub key: String,
    pub rev: String,
    /// `user` - chosen by hand; `auto` - found, or chosen by hand for the
    /// same artist under another event name.
    pub chosen: String,
}

struct EventLite {
    id: i64,
    uid: Option<String>,
    subject: Option<Subject>,
}

fn events_lite(conn: &Connection) -> AppResult<Vec<EventLite>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.uid, e.name, e.artist_team, COALESCE(ec.name, e.category)
           FROM events e LEFT JOIN event_categories ec ON ec.id = e.category_id",
    )?;
    let rows = stmt.query_map([], |r| {
        let name: String = r.get(2)?;
        let artist: Option<String> = r.get(3)?;
        let category: Option<String> = r.get(4)?;
        Ok(EventLite { id: r.get(0)?, uid: r.get(1)?, subject: subject_for(&name, artist.as_deref(), category.as_deref()) })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

struct Known {
    /// identity key -> (state, updated_at, query_key, has image)
    decisions: HashMap<String, (String, String, String, bool)>,
    /// query key -> (identity key, updated_at) of the newest picture chosen
    /// by hand for a subject.
    earlier: HashMap<String, (String, String)>,
    /// query key -> checked_at of a certain answer with a picture.
    autos: HashMap<String, String>,
}

fn known(conn: &Connection) -> AppResult<Known> {
    let mut decisions = HashMap::new();
    let mut earlier: HashMap<String, (String, String)> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT identity_key, state, updated_at, query_key, image IS NOT NULL FROM event_artworks ORDER BY updated_at, id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, bool>(4)?))
        })?;
        for row in rows {
            let (key, state, at, qk, img) = row?;
            if state == "set" && img && key.starts_with("name:") && !qk.is_empty() {
                // Ordered by time: the newest wins.
                earlier.insert(qk.clone(), (key.clone(), at.clone()));
            }
            decisions.insert(key, (state, at, qk, img));
        }
    }
    let mut autos = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT query_key, checked_at FROM artwork_lookups WHERE outcome = 'match' AND image IS NOT NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (k, at) = row?;
            autos.insert(k, at);
        }
    }
    Ok(Known { decisions, earlier, autos })
}

/// What one event shows - the order in the module doc, nothing else.
fn shown_for(event_uid: Option<&str>, subject: Option<&Subject>, k: &Known) -> Option<(String, String, &'static str)> {
    let hand = |key: &str| -> Option<Option<(String, String, &'static str)>> {
        let (state, at, _, img) = k.decisions.get(key)?;
        match state.as_str() {
            "set" if *img => Some(Some((format!("d:{key}"), at.clone(), "user"))),
            "none" => Some(None),
            // A picture that is no longer there, or "back to automatic".
            _ => None,
        }
    };
    if let Some(uid) = event_uid {
        if let Some(decided) = hand(&format!("event:{uid}")) {
            return decided;
        }
    }
    let subject = subject?;
    if let Some(decided) = hand(&subject.key) {
        return decided;
    }
    let qk = subject.query_key();
    if let Some((key, at)) = k.earlier.get(&qk) {
        return Some((format!("d:{key}"), at.clone(), "auto"));
    }
    k.autos.get(&qk).map(|at| (format!("a:{qk}"), at.clone(), "auto"))
}

pub(crate) fn artwork_map(conn: &Connection) -> AppResult<Vec<ArtworkRef>> {
    let k = known(conn)?;
    Ok(events_lite(conn)?
        .into_iter()
        .filter_map(|e| {
            shown_for(e.uid.as_deref(), e.subject.as_ref(), &k)
                .map(|(key, rev, chosen)| ArtworkRef { event_id: e.id, key, rev, chosen: chosen.to_string() })
        })
        .collect())
}

/// One picture, ready for an <img>.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkImage {
    pub key: String,
    pub rev: String,
    pub src: String,
    pub fit: String,
    /// What the picture is of, for the alt text and the tooltip.
    pub title: String,
    pub source: String,
    pub page: String,
}

/// The pictures for `keys` (from `artwork_map`). A stored picture that does
/// not read as one is left out - the screen shows the placeholder - and an
/// automatic one is searched for again.
pub(crate) fn artwork_images(conn: &Connection, keys: &[String]) -> AppResult<Vec<ArtworkImage>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for key in keys.iter().take(500) {
        if !seen.insert(key.clone()) {
            continue;
        }
        if let Some(id_key) = key.strip_prefix("d:") {
            // image, fit, label, source_title, source, source_page, updated_at
            type Chosen = (Option<String>, String, String, String, String, String, String);
            let row: Option<Chosen> = conn
                .query_row(
                    "SELECT image, fit, label, source_title, source, source_page, updated_at
                       FROM event_artworks WHERE identity_key = ?1 AND state = 'set'",
                    [id_key],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
                )
                .optional()?;
            let Some((Some(image), fit, label, title, source, page, at)) = row else { continue };
            let Some(src) = checked_data_url(&image, MAX_UPLOAD_BYTES) else { continue };
            out.push(ArtworkImage {
                key: key.clone(),
                rev: at,
                src,
                fit,
                title: if title.is_empty() { label } else { title },
                source,
                page,
            });
        } else if let Some(qk) = key.strip_prefix("a:") {
            let row: Option<(Option<String>, String, Option<String>, String)> = conn
                .query_row(
                    "SELECT image, fit, pick, checked_at FROM artwork_lookups WHERE query_key = ?1 AND outcome = 'match'",
                    [qk],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            let Some((image, fit, pick, at)) = row else { continue };
            match image.as_deref().and_then(|i| checked_data_url(i, MAX_THUMB_BYTES)) {
                Some(src) => {
                    let pick: Option<Candidate> = pick.and_then(|p| serde_json::from_str(&p).ok());
                    out.push(ArtworkImage {
                        key: key.clone(),
                        rev: at,
                        src,
                        fit,
                        title: pick.as_ref().map(|p| p.title.clone()).unwrap_or_default(),
                        source: pick.as_ref().map(|p| p.source.clone()).unwrap_or_default(),
                        page: pick.map(|p| p.page).unwrap_or_default(),
                    });
                }
                None => {
                    // Damaged: forgotten, so the next search finds it again.
                    conn.execute(
                        "UPDATE artwork_lookups SET outcome = 'failed', image = NULL, pick = NULL,
                                error = 'the kept picture was damaged', retry_after = NULL
                          WHERE query_key = ?1",
                        [qk],
                    )?;
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Suggestions: the picker on the Events page.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestInput {
    /// The event being edited - absent while a new one is typed.
    pub event_id: Option<i64>,
    pub name: String,
    pub artist_team: Option<String>,
    pub category_id: Option<i64>,
    /// `cached` - never the network; `search` - only if nothing fresh is
    /// known; `refresh` - ask again ("Search again").
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub identity_key: String,
    /// `event:<uid>` - for "only this event". Absent for an unsaved event.
    pub event_key: Option<String>,
    pub label: String,
    pub query_key: String,
    pub strategy: Strategy,
    /// Events that share the subject (this one included, once saved).
    pub shared: i64,
    /// The choice made by hand that applies now - for this event only
    /// first, then for its subject.
    pub decision: Option<Decision>,
    /// What the event shows right now: the same answer every screen gets.
    pub shown: Option<ArtworkImage>,
    /// `match`, `ask`, `none`, `failed`, `unknown` (never asked), `off`
    /// (automatic search is switched off and nothing was asked).
    pub outcome: String,
    pub level: Option<Level>,
    pub candidates: Vec<Candidate>,
    pub message: Option<String>,
    pub checked_at: Option<String>,
}

fn category_name(conn: &Connection, id: Option<i64>) -> AppResult<Option<String>> {
    match id {
        Some(id) => Ok(conn.query_row("SELECT name FROM event_categories WHERE id = ?1", [id], |r| r.get(0)).optional()?),
        None => Ok(None),
    }
}

pub(crate) fn auto_lookup_on(conn: &Connection) -> bool {
    !matches!(get_setting(conn, ARTWORK_AUTO_KEY).ok().flatten().as_deref(), Some("0"))
}

struct Planned {
    subject: Subject,
    event_uid: Option<String>,
    network: bool,
    shared: i64,
    /// "Find pictures automatically" on this computer.
    auto: bool,
}

fn plan(conn: &Connection, input: &SuggestInput, now: DateTime<Utc>) -> AppResult<Planned> {
    let category = category_name(conn, input.category_id)?;
    let subject = subject_for(&input.name, input.artist_team.as_deref(), category.as_deref())
        .ok_or_else(|| AppError::Validation("Type the event's name first - there is nothing to look for yet.".into()))?;
    let event_uid: Option<String> = match input.event_id {
        Some(id) => conn.query_row("SELECT uid FROM events WHERE id = ?1", [id], |r| r.get(0)).optional()?.flatten(),
        None => None,
    };
    let shared = events_lite(conn)?.iter().filter(|e| e.subject.as_ref().is_some_and(|s| s.key == subject.key)).count() as i64;
    let row = lookup_row(conn, &subject.query_key())?;
    let mode = input.mode.as_deref().unwrap_or("search");
    let decided_here = decision(conn, &subject.key)?.is_some_and(|d| d.state != "auto");
    let earlier = earlier_for(conn, &subject.query_key(), &subject.key)?.is_some();
    let auto = auto_lookup_on(conn);
    let network = match mode {
        "refresh" => true,
        "cached" => false,
        // A choice made by hand is reused before anything is asked.
        _ => !decided_here && !earlier && auto && due(row.as_ref(), now, true),
    };
    Ok(Planned { subject, event_uid, network, shared, auto })
}

/// Everything a suggestion needs from the database, read in one go. Pictures
/// still missing are fetched afterwards, with the database free again.
fn draft(conn: &Connection, p: &Planned, web: &dyn Web, note: Option<String>) -> AppResult<Suggestion> {
    let qk = p.subject.query_key();
    let event_key = p.event_uid.as_ref().map(|u| format!("event:{u}"));
    let own = match &event_key {
        Some(ek) => decision(conn, ek)?.filter(|d| d.state != "auto"),
        None => None,
    };
    let decided = match own {
        Some(d) => Some(d),
        None => decision(conn, &p.subject.key)?.filter(|d| d.state != "auto"),
    };
    let row = lookup_row(conn, &qk)?;
    let k = known(conn)?;
    let shown = shown_for(p.event_uid.as_deref(), Some(&p.subject), &k)
        .map(|(key, _, _)| artwork_images(conn, &[key]))
        .transpose()?
        .and_then(|mut v| v.pop());

    let mut candidates: Vec<Candidate> = Vec::new();
    if let Some(e) = earlier_for(conn, &qk, &p.subject.key)? {
        if let Some(img) = artwork_images(conn, &[format!("d:{}", e.key)])?.pop() {
            candidates.push(Candidate {
                id: format!("earlier:{}", e.key),
                source: "earlier".to_string(),
                source_id: e.key.clone(),
                title: if e.label.is_empty() { p.subject.label.clone() } else { e.label.clone() },
                description: "Chosen by hand for an event with the same artist or team".to_string(),
                page: e.source_page.clone(),
                image_url: String::new(),
                fit: img.fit.clone(),
                confidence: 0.95,
                level: Level::High,
                reasons: vec!["chosen by hand before".to_string()],
                thumb: Some(img.src),
            });
        }
    }
    if let Some(r) = &row {
        let mut list = r.candidates.clone();
        // The automatic pick's picture is kept with the answer itself.
        if let (Some(pick), true) = (&r.pick, r.has_image) {
            if let Some(img) = artwork_images(conn, &[format!("a:{qk}")])?.pop() {
                for c in list.iter_mut().filter(|c| c.id == pick.id) {
                    c.thumb = Some(img.src.clone());
                }
            }
        }
        for c in list.iter_mut().filter(|c| c.thumb.is_none()) {
            c.thumb = thumb_kept(web, &c.image_url);
        }
        candidates.extend(list);
    }
    let (outcome, level, checked_at, message) = match &row {
        Some(r) => (
            r.outcome.clone(),
            serde_json::from_value::<Level>(Value::String(r.level.clone())).ok(),
            Some(r.checked_at.clone()),
            note.or_else(|| r.error.clone()),
        ),
        None if !auto_lookup_on(conn) => ("off".to_string(), None, None, note),
        None => ("unknown".to_string(), None, None, note),
    };
    Ok(Suggestion {
        identity_key: p.subject.key.clone(),
        event_key,
        label: p.subject.label.clone(),
        query_key: qk,
        strategy: p.subject.strategy,
        shared: p.shared,
        decision: decided,
        shown,
        outcome,
        level,
        candidates,
        message,
        checked_at,
    })
}

/// The picker's one call: what is decided, what is shown, what was found -
/// asking the services only when `mode` and the cache say so. The database
/// is never held while the network is asked.
pub(crate) fn suggest_with(
    db: &Mutex<Connection>,
    web: &dyn Web,
    input: &SuggestInput,
    now: DateTime<Utc>,
) -> AppResult<Suggestion> {
    let mode = input.mode.as_deref().unwrap_or("search");
    let planned = {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        plan(&conn, input, now)?
    };
    let mut note = None;
    if planned.network {
        let qk = planned.subject.query_key();
        if let Some(_flight) = Flight::begin(&format!("{}|{qk}", web.scope()), Duration::from_secs(40)) {
            // Asked again under the flight: whoever held it may have just
            // answered this very question.
            let fresh = {
                let conn = db.lock().unwrap_or_else(|e| e.into_inner());
                lookup_row(&conn, &qk)?
            };
            if mode == "refresh" || due(fresh.as_ref(), now, true) {
                let result = search(web, &planned.subject).and_then(|c| pictured(web, c));
                note_result(web, &result);
                if let Err(e) = &result {
                    note = Some(e.message());
                }
                let conn = db.lock().unwrap_or_else(|e| e.into_inner());
                store_lookup(&conn, &qk, &result, now)?;
            }
        }
    }
    let mut out = {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        draft(&conn, &planned, web, note)?
    };
    // The pictures of what was found before, for the picker - fetched only
    // when the picker asks, not for a choice already made unless "Search
    // again" asked for alternatives, and not at all on its own while
    // automatic search is switched off.
    if mode == "refresh" || (mode == "search" && out.decision.is_none() && planned.auto) {
        out.candidates = with_thumbs(web, std::mem::take(&mut out.candidates), true);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Decisions.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideInput {
    /// The subject key, or `event:<uid>` for one event only.
    pub key: String,
    pub label: String,
    pub query_key: String,
    /// `set` - this picture; `none` - no picture; `auto` - back to automatic.
    pub state: String,
    pub image: Option<String>,
    pub fit: Option<String>,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub source_title: Option<String>,
    pub source_page: Option<String>,
    pub image_url: Option<String>,
    pub confidence: Option<f64>,
}

fn https_or_empty(s: Option<&str>) -> String {
    match s.map(str::trim) {
        Some(u) if u.starts_with("https://") && u.len() <= 500 => u.to_string(),
        _ => String::new(),
    }
}

/// Stores a choice made by hand. One row per key, written whole - so the
/// newest choice is simply the newest version of that row, here and on the
/// other computer.
pub(crate) fn decide(conn: &Connection, input: &DecideInput) -> AppResult<()> {
    if !valid_identity_key(&input.key) {
        return Err(AppError::Validation("That is not a picture TIQR can store for this event.".into()));
    }
    let query_key = if valid_query_key(&input.query_key) { input.query_key.clone() } else { String::new() };
    let source = match input.source.as_deref() {
        Some(s @ ("deezer" | "wikipedia" | "upload" | "earlier")) => s.to_string(),
        _ => String::new(),
    };
    let (image, fit) = match input.state.as_str() {
        "set" => {
            let raw = input.image.as_deref().ok_or_else(|| AppError::Validation("No picture was given.".into()))?;
            let image = checked_data_url(raw, MAX_UPLOAD_BYTES).ok_or_else(|| {
                AppError::Validation("That file is not a picture TIQR can show, or it is too large.".into())
            })?;
            let fit = match input.fit.as_deref() {
                Some("contain") => "contain",
                _ => "cover",
            };
            (Some(image), fit)
        }
        "none" | "auto" => (None, "cover"),
        other => return Err(AppError::Validation(format!("Unknown picture choice '{}'", clip_chars(other, 20)))),
    };
    let set = input.state == "set";
    let text = |v: &Option<String>, max: usize| if set { clip_chars(v.as_deref().unwrap_or_default().trim(), max) } else { String::new() };
    conn.execute(
        "INSERT INTO event_artworks
           (uid, identity_key, label, query_key, state, image, fit, source, source_id, source_title,
            source_page, image_url, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(identity_key) DO UPDATE SET
           label = excluded.label, query_key = excluded.query_key, state = excluded.state,
           image = excluded.image, fit = excluded.fit, source = excluded.source,
           source_id = excluded.source_id, source_title = excluded.source_title,
           source_page = excluded.source_page, image_url = excluded.image_url,
           confidence = excluded.confidence,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![
            artwork_uid(&input.key),
            input.key,
            clip_chars(input.label.trim(), 200),
            query_key,
            input.state,
            image,
            fit,
            if set { source } else { String::new() },
            text(&input.source_id, 100),
            text(&input.source_title, 200),
            if set { https_or_empty(input.source_page.as_deref()) } else { String::new() },
            if set { https_or_empty(input.image_url.as_deref()) } else { String::new() },
            if set { input.confidence.filter(|c| c.is_finite()).map(|c| c.clamp(0.0, 1.0)) } else { None },
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Finding pictures for events that have none, a few at a time.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillReport {
    pub searched: i64,
    pub found: i64,
    pub to_confirm: i64,
    pub failed: i64,
    /// Searches still waiting after this round.
    pub remaining: i64,
    /// Switched off on this computer, or resting after "no connection".
    pub paused: bool,
}

/// The searches events are waiting for, soonest event first: an upcoming
/// event is the one whose picture is looked at.
fn waiting_searches(conn: &Connection, now: DateTime<Utc>) -> AppResult<Vec<Subject>> {
    let k = known(conn)?;
    let mut stmt = conn.prepare(
        "SELECT e.id, e.uid, e.name, e.artist_team, COALESCE(ec.name, e.category)
           FROM events e LEFT JOIN event_categories ec ON ec.id = e.category_id
          ORDER BY (e.status <> 'upcoming'),
                   CASE WHEN e.status = 'upcoming' THEN IFNULL(e.event_date, '9999') END ASC,
                   e.event_date DESC, e.id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, Option<String>>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, Option<String>>(4)?))
    })?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for row in rows {
        let (uid, name, artist, category) = row?;
        let Some(subject) = subject_for(&name, artist.as_deref(), category.as_deref()) else { continue };
        // Decided by hand (a picture or "none"), or already shown: nothing to ask.
        let decided = |key: &str| k.decisions.get(key).is_some_and(|(s, ..)| s != "auto");
        if uid.as_deref().is_some_and(|u| decided(&format!("event:{u}"))) || decided(&subject.key) {
            continue;
        }
        if shown_for(uid.as_deref(), Some(&subject), &k).is_some() {
            continue;
        }
        let qk = subject.query_key();
        if !seen.insert(qk.clone()) {
            continue;
        }
        if due(lookup_row(conn, &qk)?.as_ref(), now, false) {
            out.push(subject);
        }
    }
    Ok(out)
}

pub(crate) fn backfill_with(
    db: &Mutex<Connection>,
    web: &dyn Web,
    max: usize,
    now: DateTime<Utc>,
) -> AppResult<BackfillReport> {
    let waiting = {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if !auto_lookup_on(&conn) {
            return Ok(BackfillReport { paused: true, ..Default::default() });
        }
        waiting_searches(&conn, now)?
    };
    let mut report = BackfillReport { remaining: waiting.len() as i64, ..Default::default() };
    if resting(web) {
        report.paused = true;
        return Ok(report);
    }
    for subject in waiting.iter().take(max) {
        let qk = subject.query_key();
        let Some(_flight) = Flight::begin(&format!("{}|{qk}", web.scope()), Duration::from_secs(1)) else { continue };
        let result = search(web, subject).and_then(|c| pictured(web, c));
        note_result(web, &result);
        {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            store_lookup(&conn, &qk, &result, now)?;
            match lookup_row(&conn, &qk)?.map(|r| r.outcome) {
                Some(o) if o == "match" => report.found += 1,
                Some(o) if o == "ask" => report.to_confirm += 1,
                Some(o) if o == "failed" => report.failed += 1,
                _ => {}
            }
        }
        report.searched += 1;
        report.remaining -= 1;
        if matches!(&result, Err(e) if e.is_offline()) {
            report.paused = true;
            break;
        }
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Commands.
// ---------------------------------------------------------------------------

/// Off the window's thread too: every screen asks for it, and with thousands
/// of events it is worth a few milliseconds the window should not wait for.
#[tauri::command(async)]
pub fn event_artwork_map(state: State<'_, AppState>) -> AppResult<Vec<ArtworkRef>> {
    let conn = state.db.lock().unwrap();
    artwork_map(&conn)
}

#[tauri::command(async)]
pub fn event_artwork_images(state: State<'_, AppState>, keys: Vec<String>) -> AppResult<Vec<ArtworkImage>> {
    let conn = state.db.lock().unwrap();
    artwork_images(&conn, &keys)
}

/// Network, so never on the window's thread: the whole call runs on a
/// blocking worker, and takes the database only for its reads and writes.
#[tauri::command]
pub async fn event_artwork_suggest(app: tauri::AppHandle, input: SuggestInput) -> AppResult<Suggestion> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        suggest_with(&state.db, &LiveWeb::new(), &input, Utc::now())
    })
    .await
    .map_err(|e| AppError::Other(format!("the picture search stopped: {e}")))?
}

#[tauri::command]
pub fn event_artwork_decide(state: State<AppState>, input: DecideInput) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    decide(&conn, &input)
}

#[tauri::command]
pub async fn event_artwork_backfill(app: tauri::AppHandle, max: Option<u32>) -> AppResult<BackfillReport> {
    let max = max.unwrap_or(3).clamp(1, 10) as usize;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        backfill_with(&state.db, &LiveWeb::new(), max, Utc::now())
    })
    .await
    .map_err(|e| AppError::Other(format!("the picture search stopped: {e}")))?
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkOptions {
    /// Look for pictures on its own - for new events as they are typed, and
    /// for events that have none, a few at a time.
    pub auto: bool,
}

#[tauri::command]
pub fn get_event_artwork_options(state: State<AppState>) -> AppResult<ArtworkOptions> {
    let conn = state.db.lock().unwrap();
    Ok(ArtworkOptions { auto: auto_lookup_on(&conn) })
}

#[tauri::command]
pub fn set_event_artwork_options(state: State<AppState>, options: ArtworkOptions) -> AppResult<ArtworkOptions> {
    let conn = state.db.lock().unwrap();
    set_setting(&conn, ARTWORK_AUTO_KEY, if options.auto { "1" } else { "0" })?;
    Ok(ArtworkOptions { auto: auto_lookup_on(&conn) })
}

#[cfg(test)]
mod tests {
    //! Deterministic: every answer from a "service" is a fixture shaped like
    //! the real one, every picture a real one-pixel PNG, and each test's fake
    //! network has its own scope, so pace, pauses and kept pictures never
    //! leak from one test into another running beside it.
    use super::*;
    use crate::db::test_conn;
    use chrono::TimeZone;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A real 1x1 PNG.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D,
        0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn png_url() -> String {
        data_url(PNG).unwrap()
    }

    type Answer = Box<dyn Fn(&str) -> Result<Value, WebError> + Sync>;

    struct FakeWeb {
        scope: String,
        answer: Answer,
        pictures_fail: bool,
        delay: Duration,
        calls: Mutex<Vec<String>>,
    }

    static SCOPES: AtomicUsize = AtomicUsize::new(0);

    impl FakeWeb {
        fn new(answer: impl Fn(&str) -> Result<Value, WebError> + Sync + 'static) -> Self {
            FakeWeb {
                scope: format!("test-{}", SCOPES.fetch_add(1, Ordering::SeqCst)),
                answer: Box::new(answer),
                pictures_fail: false,
                delay: Duration::ZERO,
                calls: Mutex::new(Vec::new()),
            }
        }
        /// No connection at all - and it counts every attempt.
        fn offline() -> Self {
            let mut w = FakeWeb::new(|_| Err(WebError::Offline));
            w.pictures_fail = true;
            w
        }
        fn json_calls(&self) -> usize {
            self.calls.lock().unwrap().iter().filter(|c| c.starts_with("json ")).count()
        }
        fn picture_calls(&self) -> usize {
            self.calls.lock().unwrap().iter().filter(|c| c.starts_with("picture ")).count()
        }
        fn calls_to(&self, part: &str) -> usize {
            self.calls.lock().unwrap().iter().filter(|c| c.contains(part)).count()
        }
    }

    impl Web for FakeWeb {
        fn json(&self, url: &str) -> Result<Value, WebError> {
            self.calls.lock().unwrap().push(format!("json {url}"));
            if !self.delay.is_zero() {
                std::thread::sleep(self.delay);
            }
            (self.answer)(url)
        }
        fn picture(&self, url: &str) -> Result<Vec<u8>, WebError> {
            self.calls.lock().unwrap().push(format!("picture {url}"));
            if !picture_host_allowed(url) {
                return Err(WebError::NotAllowed);
            }
            if self.pictures_fail {
                return Err(WebError::Offline);
            }
            Ok(PNG.to_vec())
        }
        fn scope(&self) -> &str {
            &self.scope
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 24, 12, 0, 0).unwrap()
    }

    fn db() -> Mutex<Connection> {
        Mutex::new(test_conn())
    }

    fn category_id(conn: &Connection, name: &str) -> i64 {
        conn.query_row("SELECT id FROM event_categories WHERE name = ?1", [name], |r| r.get(0))
            .optional()
            .unwrap()
            .unwrap_or_else(|| {
                conn.execute("INSERT INTO event_categories (name) VALUES (?1)", [name]).unwrap();
                conn.last_insert_rowid()
            })
    }

    fn add_event(conn: &Connection, name: &str, category: Option<&str>, date: &str) -> i64 {
        let cat = category.map(|c| category_id(conn, c));
        conn.execute(
            "INSERT INTO events (name, event_date, category, category_id) VALUES (?1, ?2, ?3, ?4)",
            params![name, date, category, cat],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn uid_of(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT uid FROM events WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    fn deezer_artist(id: i64, name: &str, fans: i64) -> Value {
        json!({
            "id": id, "name": name, "link": format!("https://www.deezer.com/artist/{id}"),
            "picture": format!("https://api.deezer.com/artist/{id}/image"),
            "picture_small": format!("https://e-cdns-images.dzcdn.net/images/artist/h{id}/56x56-000000-80-0-0.jpg"),
            "picture_medium": format!("https://e-cdns-images.dzcdn.net/images/artist/h{id}/250x250-000000-80-0-0.jpg"),
            "nb_album": 12, "nb_fan": fans, "radio": true, "type": "artist"
        })
    }

    fn deezer_answer(artists: Vec<Value>) -> Value {
        json!({ "data": artists, "total": 2 })
    }

    fn coldplay() -> Value {
        deezer_answer(vec![deezer_artist(892, "Coldplay", 25_300_000)])
    }

    fn nothing_on_wikipedia() -> Value {
        json!({ "batchcomplete": true })
    }

    fn wiki_page(index: i64, page_id: i64, title: &str, description: &str, thumb: Option<&str>, qid: &str) -> Value {
        let mut p = json!({
            "pageid": page_id, "ns": 0, "title": title, "index": index,
            "description": description, "descriptionsource": "local",
            "pageprops": { "wikibase_item": qid }
        });
        if let Some(t) = thumb {
            p["thumbnail"] = json!({ "source": t, "width": 250, "height": 250 });
            p["pageimage"] = json!(t.rsplit('/').next().unwrap_or_default());
        }
        p
    }

    fn wiki_answer(pages: Vec<Value>) -> Value {
        json!({ "batchcomplete": true, "query": { "pages": pages } })
    }

    fn thumb(name: &str) -> String {
        format!("https://upload.wikimedia.org/wikipedia/en/thumb/a/ab/{name}/250px-{name}.png")
    }

    /// Music on Deezer, nothing on Wikipedia.
    fn music(deezer: Value) -> FakeWeb {
        FakeWeb::new(move |url| if url.contains("api.deezer.com") { Ok(deezer.clone()) } else { Ok(nothing_on_wikipedia()) })
    }

    fn input(id: Option<i64>, name: &str, category: Option<i64>, mode: &str) -> SuggestInput {
        SuggestInput {
            event_id: id,
            name: name.to_string(),
            artist_team: None,
            category_id: category,
            mode: Some(mode.to_string()),
        }
    }

    fn choose(key: &str, query_key: &str, image: Option<String>, state: &str) -> DecideInput {
        DecideInput {
            key: key.to_string(),
            label: "Coldplay".to_string(),
            query_key: query_key.to_string(),
            state: state.to_string(),
            image,
            fit: Some("cover".to_string()),
            source: Some("upload".to_string()),
            source_id: None,
            source_title: Some("My own photo".to_string()),
            source_page: None,
            image_url: None,
            confidence: None,
        }
    }

    fn map_of(db: &Mutex<Connection>) -> HashMap<i64, ArtworkRef> {
        artwork_map(&db.lock().unwrap()).unwrap().into_iter().map(|r| (r.event_id, r)).collect()
    }

    // -- names ---------------------------------------------------------------

    #[test]
    fn every_night_of_one_artist_is_one_subject_and_a_similar_name_is_another() {
        let s = |n: &str| subject_for(n, None, Some("Concert")).unwrap();
        let coldplay = s("Coldplay");
        assert_eq!(coldplay.key, "name:coldplay");
        assert_eq!(coldplay.query_key(), "artist:coldplay");
        for same in ["Coldplay (2026)", "COLDPLAY 2026", "Coldplay tickets", "  Coldplay  ", "Coldplay [rescheduled]"] {
            assert_eq!(s(same).key, coldplay.key, "{same}");
        }
        let tribute = s("Coldplay Tribute Band");
        assert_ne!(tribute.key, coldplay.key);
        assert_ne!(tribute.query_key(), coldplay.query_key(), "a similar name is never the same search");
        // Two subjects, one search: the services are asked once for both.
        let prague = s("Coldplay - Prague");
        assert_ne!(prague.key, coldplay.key);
        assert_eq!(prague.query_key(), coldplay.query_key());
        assert_eq!(s("Imagine Dragons World Tour 2026").query, "Imagine Dragons");
        assert_eq!(s("Concert: Dua Lipa").query, "Dua Lipa");
        assert_eq!(s("Florence + The Machine").key, s("Florence and the Machine").key);
        // A name that is nothing but a year is not emptied.
        assert_eq!(s("The 1975").key, "name:the 1975");
        assert!(subject_for("   ", None, None).is_none());
        assert!(subject_for("2026", None, None).is_some(), "kept as it is rather than lost");
        // artist_team wins when it is filled in.
        assert_eq!(subject_for("Friday night", Some("Muse"), Some("Concert")).unwrap().key, "name:muse");
    }

    #[test]
    fn two_matches_of_one_team_are_two_subjects() {
        let m1 = subject_for("Real Madrid vs Barcelona", None, Some("Sports")).unwrap();
        let m2 = subject_for("Real Madrid vs Bayern Munich", None, Some("Sports")).unwrap();
        assert_ne!(m1.key, m2.key);
        assert_eq!(m1.strategy, Strategy::Team);
        assert_eq!(m1.query, "Real Madrid");
        assert_eq!(m1.label, "Real Madrid vs Barcelona");
        assert_eq!(m1.query_key(), m2.query_key(), "the home side's picture is found once");
        // " vs " needs no category.
        assert_eq!(subject_for("Real Madrid vs Barcelona", None, None).unwrap().strategy, Strategy::Team);
        // " - " is a match only where the category says sport...
        let derby = subject_for("Slovan Bratislava - Spartak Trnava", None, Some("Football")).unwrap();
        assert_eq!((derby.strategy, derby.query.as_str(), derby.sport), (Strategy::Team, "Slovan Bratislava", Some("football")));
        let unknown = subject_for("Slovan Bratislava - Spartak Trnava", None, None).unwrap();
        assert_eq!(unknown.strategy, Strategy::Any);
        // ...and " v " is Slovak for "in".
        let k = subject_for("Koncert v Bratislave", None, Some("Koncert")).unwrap();
        assert_eq!((k.strategy, k.query.as_str()), (Strategy::Artist, "Koncert v Bratislave"));
        assert_eq!(subject_for("Pohoda Festival 2026", None, Some("Festival")).unwrap().strategy, Strategy::Show);
        assert_eq!(subject_for("Monaco Grand Prix", None, None).unwrap().strategy, Strategy::Show);
    }

    #[test]
    fn names_in_any_language_never_break_the_split() {
        let names = [
            "Ľudovít Štúr – Žilina", "Ünïcödé vs Ŝçřïpţ", "Αθήνα vs Θεσσαλονίκη", "東京 - 大阪", "İstanbul vs Ankara",
            "🎸 - 🎤", "ß vs SS", " - ", "vs", "a vs ", "(())", "2026/27", "   ", "Koncert v. Praha", "x – – y",
            "Beyoncé: Cowboy Carter", "Sigur Rós @ O2", "Mötley Crüe feat. İzmir", ": : :", "— vs —",
        ];
        for n in names {
            for c in [None, Some("Sports"), Some("Concert"), Some("Football"), Some("Festival")] {
                if let Some(s) = subject_for(n, None, c) {
                    assert!(valid_identity_key(&s.key), "{n}: {}", s.key);
                    assert!(valid_query_key(&s.query_key()) || norm(&s.query).is_empty(), "{n}");
                }
            }
        }
        assert_eq!(subject_for("İstanbul vs Ankara", None, None).unwrap().query, "İstanbul");
        assert_eq!(norm("Ce\u{301}line Dion"), norm("Céline Dion"));
        assert_eq!(norm("İstanbul"), "istanbul");
    }

    // -- confidence ------------------------------------------------------------

    fn artist(id: i64, name: &str, fans: i64) -> DeezerArtist {
        DeezerArtist {
            id,
            name: name.to_string(),
            link: format!("https://www.deezer.com/artist/{id}"),
            picture: Some(format!("https://e-cdns-images.dzcdn.net/images/artist/h{id}/250x250.jpg")),
            fans,
        }
    }

    #[test]
    fn a_famous_artist_with_a_name_of_its_own_is_certain_and_a_longer_name_is_not_it() {
        let c = score_deezer("Coldplay", Strategy::Artist, &[artist(892, "Coldplay", 25_300_000), artist(5, "Coldplay Tribute Band", 120)]);
        assert_eq!((c[0].id.as_str(), c[0].level), ("deezer:892", Level::High));
        assert_eq!(c[1].level, Level::Low);
        assert!(c[0].reasons.iter().any(|r| r.contains("only artist")));
        assert_eq!(c[0].page, "https://www.deezer.com/artist/892");
    }

    #[test]
    fn two_artists_with_one_name_wait_for_a_yes() {
        let c = score_deezer("Muse", Strategy::Artist, &[artist(1, "Muse", 3_000_000), artist(2, "MUSE", 900_000)]);
        assert_eq!(c[0].level, Level::Medium);
        assert_eq!(c[1].level, Level::Low);
    }

    #[test]
    fn without_a_category_only_a_famous_artist_is_certain() {
        let list = [artist(1, "Arsenal", 120_000), artist(2, "Arsenal", 300)];
        assert_eq!(score_deezer("Arsenal", Strategy::Artist, &list)[0].level, Level::High);
        assert_eq!(score_deezer("Arsenal", Strategy::Any, &list)[0].level, Level::Medium);
        assert_eq!(score_deezer("Coldplay", Strategy::Any, &[artist(892, "Coldplay", 25_300_000)])[0].level, Level::High);
        assert_eq!(score_deezer("Obscure", Strategy::Artist, &[artist(9, "Obscure", 40)])[0].level, Level::Medium);
    }

    #[test]
    fn deezer_answers_are_read_for_what_they_mean() {
        assert_eq!(parse_deezer(&json!({"error": {"type": "Exception", "message": "Quota limit exceeded", "code": 4}})).err(), Some(WebError::Busy(10)));
        assert!(parse_deezer(&json!({"error": {"code": 800}})).unwrap().is_empty());
        assert_eq!(parse_deezer(&json!({"unexpected": true})).err(), Some(WebError::Malformed));
        assert_eq!(parse_deezer(&json!([1, 2])).err(), Some(WebError::Malformed));
        let mut no_picture = deezer_artist(7, "Nobody", 5);
        no_picture["picture_medium"] = json!("https://e-cdns-images.dzcdn.net/images/artist//250x250-000000-80-0-0.jpg");
        let parsed = parse_deezer(&deezer_answer(vec![no_picture, json!({"id": "x"})])).unwrap();
        assert_eq!(parsed.len(), 1, "an entry without an id is skipped");
        assert!(parsed[0].picture.is_none(), "Deezer's empty picture is no picture");
        assert!(score_deezer("Nobody", Strategy::Artist, &parsed).is_empty(), "never offered without a picture");
    }

    #[test]
    fn a_club_is_found_through_its_decoration_and_a_city_of_its_name_is_no_rival() {
        let pages = parse_wikipedia(&wiki_answer(vec![
            wiki_page(1, 17_000, "Liverpool", "City in Merseyside, England", Some("https://upload.wikimedia.org/x/Liverpool.jpg"), "Q24826"),
            wiki_page(2, 18_119, "Liverpool F.C.", "Association football club in England", Some(&thumb("Liverpool_FC.svg")), "Q1130849"),
            wiki_page(3, 18_200, "Liverpool (disambiguation)", "", None, "Q1"),
        ]))
        .unwrap();
        let c = score_wikipedia("Liverpool", Strategy::Team, &pages);
        let club = c.iter().find(|c| c.title == "Liverpool F.C.").unwrap();
        assert_eq!(club.level, Level::High);
        assert_eq!(club.fit, "contain", "a crest is shown whole");
        assert_eq!(club.id, "wikipedia:Q1130849");
        assert_eq!(c.iter().find(|c| c.title == "Liverpool").unwrap().level, Level::Low);
        assert_eq!(c.len(), 2, "a page without a picture is never offered");
    }

    #[test]
    fn two_pages_that_both_fit_are_never_certain() {
        let pages = parse_wikipedia(&wiki_answer(vec![
            wiki_page(1, 1, "Rock for People", "Music festival in the Czech Republic", Some(&thumb("RfP.svg")), "Q2"),
            wiki_page(2, 2, "Rock for People (2009 festival)", "", Some(&thumb("RfP2009.svg")), "Q3"),
        ]))
        .unwrap();
        let c = score_wikipedia("Rock for People", Strategy::Show, &pages);
        assert!(c.iter().all(|c| c.level == Level::Medium), "{c:?}");
    }

    #[test]
    fn a_disambiguation_page_is_never_offered_and_odd_answers_are_errors() {
        let mut d = wiki_page(1, 1, "Mercury", "Topics referred to by the same term", Some(&thumb("M.svg")), "Q4");
        d["pageprops"]["disambiguation"] = json!("");
        let pages = parse_wikipedia(&wiki_answer(vec![d])).unwrap();
        assert!(score_wikipedia("Mercury", Strategy::Any, &pages).is_empty());
        assert_eq!(parse_wikipedia(&json!({"error": {"code": "ratelimited"}})).err(), Some(WebError::Busy(30)));
        assert!(parse_wikipedia(&nothing_on_wikipedia()).unwrap().is_empty());
        assert_eq!(parse_wikipedia(&json!([])).err(), Some(WebError::Malformed));
        assert_eq!(parse_wikipedia(&json!({"query": {}})).err(), Some(WebError::Malformed));
    }

    #[test]
    fn pictures_come_only_from_the_services_own_picture_servers() {
        assert!(picture_host_allowed("https://e-cdns-images.dzcdn.net/images/artist/x/250x250.jpg"));
        assert!(picture_host_allowed("https://cdn-images.dzcdn.net/images/artist/x/250x250.jpg"));
        assert!(picture_host_allowed("https://upload.wikimedia.org/wikipedia/en/x.png"));
        for bad in [
            "http://upload.wikimedia.org/x.png",
            "https://upload.wikimedia.org.evil.example/x.png",
            "https://dzcdn.net.evil.example/x.jpg",
            "https://evil.example/x.jpg",
            "https://127.0.0.1/x.png",
            "file:///etc/passwd",
            "not an address",
        ] {
            assert!(!picture_host_allowed(bad), "{bad}");
        }
    }

    #[test]
    fn only_a_real_picture_can_be_chosen() {
        let conn = test_conn();
        let png = png_url();
        assert!(decide(&conn, &choose("name:coldplay", "artist:coldplay", Some(png.clone()), "set")).is_ok());
        for bad in [
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "data:image/png;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "https://example.com/x.png",
            "data:image/png;base64,%%%",
        ] {
            assert!(decide(&conn, &choose("name:coldplay", "artist:coldplay", Some(bad.to_string()), "set")).is_err(), "{bad}");
        }
        let huge = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode([PNG, &vec![0u8; MAX_UPLOAD_BYTES]].concat()));
        assert!(decide(&conn, &choose("name:coldplay", "artist:coldplay", Some(huge), "set")).is_err());
        for key in ["name:Coldplay", "name:", "event:../../x", "coldplay", "event:"] {
            assert!(decide(&conn, &choose(key, "artist:coldplay", Some(png.clone()), "set")).is_err(), "{key}");
        }
        assert!(decide(&conn, &choose("name:coldplay", "artist:coldplay", None, "delete")).is_err());
        assert_eq!(https_or_empty(Some("javascript:alert(1)")), "");
        assert_eq!(https_or_empty(Some("https://www.deezer.com/artist/892")), "https://www.deezer.com/artist/892");
        // The rejected attempts changed nothing.
        let stored: String = conn.query_row("SELECT image FROM event_artworks", [], |r| r.get(0)).unwrap();
        assert_eq!(stored, png);
    }

    #[test]
    fn a_subject_is_named_the_same_way_on_every_computer() {
        let u = artwork_uid("name:coldplay");
        assert_eq!(u, artwork_uid("name:coldplay"));
        assert_ne!(u, artwork_uid("name:muse"));
        assert_eq!(u.len(), 32);
        assert!(u.chars().all(|c| c.is_ascii_hexdigit()));
        let (a, b) = (test_conn(), test_conn());
        decide(&a, &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        decide(&b, &choose("name:coldplay", "artist:coldplay", None, "none")).unwrap();
        let ua: String = a.query_row("SELECT uid FROM event_artworks", [], |r| r.get(0)).unwrap();
        let ub: String = b.query_row("SELECT uid FROM event_artworks", [], |r| r.get(0)).unwrap();
        assert_eq!(ua, ub);
        assert_eq!(ua, u);
    }

    // -- the whole way through -------------------------------------------------

    #[test]
    fn five_nights_of_one_artist_share_one_picture_found_with_one_search() {
        let db = db();
        let ids: Vec<i64> = {
            let c = db.lock().unwrap();
            (1..=5).map(|d| add_event(&c, "Coldplay", Some("Concert"), &format!("2026-11-0{d}"))).collect()
        };
        let web = music(coldplay());
        let r = backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!((r.searched, r.found, r.remaining), (1, 1, 0));
        assert_eq!(web.json_calls(), 1, "Deezer was certain - nothing else was asked");
        assert_eq!(web.picture_calls(), 1);
        let map = map_of(&db);
        for id in &ids {
            assert_eq!(map[id].key, "a:artist:coldplay");
            assert_eq!(map[id].chosen, "auto");
        }
        // Asked again: nothing to do, nothing asked.
        assert_eq!(backfill_with(&db, &web, 5, now()).unwrap().searched, 0);
        assert_eq!(web.json_calls(), 1);
        // Every screen gets the same picture: one key, one image.
        let images = artwork_images(&db.lock().unwrap(), &["a:artist:coldplay".to_string()]).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src, png_url());
        assert_eq!((images[0].source.as_str(), images[0].title.as_str()), ("deezer", "Coldplay"));
        assert_eq!(images[0].page, "https://www.deezer.com/artist/892");
    }

    #[test]
    fn what_is_known_is_shown_without_the_network() {
        let db = db();
        let (id, concert) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Coldplay", Some("Concert"), "2026-11-01"), category_id(&c, "Concert"))
        };
        backfill_with(&db, &music(coldplay()), 5, now()).unwrap();
        // From here on there is no internet - and nothing tries it.
        let offline = FakeWeb::offline();
        for mode in ["cached", "search"] {
            let s = suggest_with(&db, &offline, &input(Some(id), "Coldplay", Some(concert), mode), now()).unwrap();
            assert_eq!(s.outcome, "match", "{mode}");
            assert_eq!(s.shown.as_ref().map(|i| i.src.clone()), Some(png_url()));
            assert_eq!(s.candidates[0].thumb, Some(png_url()), "the pick's picture is kept with it");
        }
        assert_eq!(offline.json_calls() + offline.picture_calls(), 0);
        assert_eq!(backfill_with(&db, &offline, 5, now()).unwrap().searched, 0);
        assert_eq!(offline.json_calls(), 0);
    }

    #[test]
    fn nothing_found_leaves_the_placeholder() {
        let db = db();
        let id = add_event(&db.lock().unwrap(), "Unknown Local Band", Some("Concert"), "2026-11-01");
        let web = music(deezer_answer(vec![]));
        let r = backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!((r.searched, r.found, r.to_confirm), (1, 0, 0));
        assert!(!map_of(&db).contains_key(&id));
        let row = lookup_row(&db.lock().unwrap(), "artist:unknown local band").unwrap().unwrap();
        assert_eq!(row.outcome, "none");
        // Not asked again for two weeks on its own.
        assert_eq!(backfill_with(&db, &web, 5, now() + Span::days(3)).unwrap().searched, 0);
        assert_eq!(backfill_with(&db, &web, 5, now() + Span::days(15)).unwrap().searched, 1);
    }

    #[test]
    fn an_ambiguous_name_waits_for_a_yes_and_the_yes_shows_on_every_night() {
        let db = db();
        let (a, b, concert) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Muse", Some("Concert"), "2026-11-01"), add_event(&c, "Muse", Some("Concert"), "2026-11-02"), category_id(&c, "Concert"))
        };
        let web = music(deezer_answer(vec![deezer_artist(1, "Muse", 3_000_000), deezer_artist(2, "MUSE", 900_000)]));
        let r = backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!(r.to_confirm, 1);
        assert!(map_of(&db).is_empty(), "a likely answer is never shown without a yes");

        let s = suggest_with(&db, &web, &input(Some(a), "Muse", Some(concert), "search"), now()).unwrap();
        assert_eq!((s.outcome.as_str(), s.level), ("ask", Some(Level::Medium)));
        let pick = s.candidates[0].clone();
        assert!(pick.thumb.is_some());
        decide(
            &db.lock().unwrap(),
            &DecideInput {
                key: s.identity_key.clone(),
                label: s.label.clone(),
                query_key: s.query_key.clone(),
                state: "set".into(),
                image: pick.thumb.clone(),
                fit: Some(pick.fit.clone()),
                source: Some(pick.source.clone()),
                source_id: Some(pick.source_id.clone()),
                source_title: Some(pick.title.clone()),
                source_page: Some(pick.page.clone()),
                image_url: Some(pick.image_url.clone()),
                confidence: Some(pick.confidence),
            },
        )
        .unwrap();
        let map = map_of(&db);
        assert_eq!(map[&a].key, "d:name:muse");
        assert_eq!(map[&b].key, "d:name:muse");
        assert_eq!(map[&a].chosen, "user");
        let stored: (String, String, f64) = db
            .lock()
            .unwrap()
            .query_row("SELECT source, source_id, confidence FROM event_artworks", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((stored.0.as_str(), stored.1.as_str()), ("deezer", "1"), "the source is kept with the picture");
    }

    #[test]
    fn a_late_search_never_replaces_a_choice_made_by_hand() {
        let db = db();
        let (id, concert) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Coldplay", Some("Concert"), "2026-11-01"), category_id(&c, "Concert"))
        };
        decide(&db.lock().unwrap(), &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        // A search that comes back afterwards, certain of another picture.
        let s = suggest_with(&db, &music(coldplay()), &input(Some(id), "Coldplay", Some(concert), "refresh"), now()).unwrap();
        assert_eq!(s.outcome, "match");
        assert_eq!(s.decision.as_ref().map(|d| d.state.as_str()), Some("set"));
        let map = map_of(&db);
        assert_eq!((map[&id].key.as_str(), map[&id].chosen.as_str()), ("d:name:coldplay", "user"));
        let source: String = db.lock().unwrap().query_row("SELECT source FROM event_artworks", [], |r| r.get(0)).unwrap();
        assert_eq!(source, "upload");
        // A search the backfill would make is not even made.
        let web = music(coldplay());
        assert_eq!(backfill_with(&db, &web, 5, now()).unwrap().searched, 0);
        assert_eq!(web.json_calls(), 0);
    }

    #[test]
    fn unrelated_events_never_share_a_picture() {
        let db = db();
        let (coldplay_id, tribute_id) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Coldplay", Some("Concert"), "2026-11-01"), add_event(&c, "Coldplay Tribute Band", Some("Concert"), "2026-11-02"))
        };
        decide(&db.lock().unwrap(), &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        let map = map_of(&db);
        assert!(map.contains_key(&coldplay_id));
        assert!(!map.contains_key(&tribute_id));
    }

    #[test]
    fn the_same_artist_under_another_event_name_gets_the_hand_picked_picture_without_a_search() {
        let db = db();
        let (home, away) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Coldplay", Some("Concert"), "2026-11-01"), add_event(&c, "Coldplay - Vienna", Some("Concert"), "2026-11-05"))
        };
        decide(&db.lock().unwrap(), &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        let map = map_of(&db);
        assert_eq!((map[&home].chosen.as_str(), map[&away].chosen.as_str()), ("user", "auto"));
        assert_eq!(map[&away].key, "d:name:coldplay");
        let web = music(coldplay());
        assert_eq!(backfill_with(&db, &web, 5, now()).unwrap().searched, 0);
        assert_eq!(web.json_calls(), 0, "a confirmed picture is reused before anything is asked");
        // "No picture" for one subject is never carried to another.
        decide(&db.lock().unwrap(), &choose("name:coldplay", "artist:coldplay", None, "none")).unwrap();
        let map = map_of(&db);
        assert!(!map.contains_key(&home));
        assert!(!map.contains_key(&away));
    }

    #[test]
    fn a_picture_for_one_event_only_leaves_the_other_nights_alone() {
        let db = db();
        let ids: Vec<i64> = {
            let c = db.lock().unwrap();
            (1..=3).map(|d| add_event(&c, "Coldplay", Some("Concert"), &format!("2026-11-0{d}"))).collect()
        };
        let (u2, u3) = { let c = db.lock().unwrap(); (uid_of(&c, ids[1]), uid_of(&c, ids[2])) };
        let c = db.lock().unwrap();
        decide(&c, &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        decide(&c, &choose(&format!("event:{u2}"), "artist:coldplay", Some(png_url()), "set")).unwrap();
        decide(&c, &choose(&format!("event:{u3}"), "artist:coldplay", None, "none")).unwrap();
        drop(c);
        let map = map_of(&db);
        assert_eq!(map[&ids[0]].key, "d:name:coldplay");
        assert_eq!(map[&ids[1]].key, format!("d:event:{u2}"));
        assert!(!map.contains_key(&ids[2]), "no picture for this night, by choice");
        // A picture meant for one event is never offered to another subject.
        let c = db.lock().unwrap();
        decide(&c, &choose("name:coldplay", "artist:coldplay", None, "auto")).unwrap();
        drop(c);
        assert!(!map_of(&db).contains_key(&ids[0]));
    }

    #[test]
    fn no_connection_is_remembered_and_tried_again_later() {
        let db = db();
        let id = add_event(&db.lock().unwrap(), "Coldplay", Some("Concert"), "2026-11-01");
        let offline = FakeWeb::offline();
        let r = backfill_with(&db, &offline, 5, now()).unwrap();
        assert_eq!((r.searched, r.failed, r.paused), (1, 1, true));
        let row = lookup_row(&db.lock().unwrap(), "artist:coldplay").unwrap().unwrap();
        assert_eq!(row.outcome, "failed");
        assert!(row.retry_after.is_some());
        assert!(!map_of(&db).contains_key(&id), "a placeholder, not an error");
        // Straight away again: resting, nothing tried.
        let calls = offline.json_calls();
        assert!(backfill_with(&db, &offline, 5, now()).unwrap().paused);
        assert_eq!(offline.json_calls(), calls);
        // Later, with the connection back: found.
        let back = music(coldplay());
        let r = backfill_with(&db, &back, 5, now() + Span::minutes(11)).unwrap();
        assert_eq!(r.found, 1);
        assert!(map_of(&db).contains_key(&id));
    }

    #[test]
    fn a_failure_never_replaces_a_certain_answer() {
        let db = db();
        let (id, concert) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Coldplay", Some("Concert"), "2026-11-01"), category_id(&c, "Concert"))
        };
        backfill_with(&db, &music(coldplay()), 5, now()).unwrap();
        let s = suggest_with(&db, &FakeWeb::offline(), &input(Some(id), "Coldplay", Some(concert), "refresh"), now()).unwrap();
        assert!(s.message.unwrap().contains("No internet"));
        assert_eq!(s.outcome, "match");
        assert_eq!(map_of(&db)[&id].key, "a:artist:coldplay");
        let row = lookup_row(&db.lock().unwrap(), "artist:coldplay").unwrap().unwrap();
        assert!(row.has_image);
        assert_eq!(row.attempts, 1);
    }

    #[test]
    fn a_found_picture_that_cannot_be_downloaded_is_tried_again_soon() {
        let db = db();
        let id = add_event(&db.lock().unwrap(), "Coldplay", Some("Concert"), "2026-11-01");
        let mut web = music(coldplay());
        web.pictures_fail = true;
        let r = backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!(r.failed, 1);
        assert!(!map_of(&db).contains_key(&id));
        let row = lookup_row(&db.lock().unwrap(), "artist:coldplay").unwrap().unwrap();
        assert_eq!(row.outcome, "failed", "not 'nothing found', which would wait two weeks");
        assert_eq!(backfill_with(&db, &music(coldplay()), 5, now() + Span::minutes(11)).unwrap().found, 1);
    }

    #[test]
    fn a_service_that_asks_to_slow_down_is_left_alone() {
        let db = db();
        add_event(&db.lock().unwrap(), "Coldplay", Some("Concert"), "2026-11-01");
        let web = FakeWeb::new(|url| if url.contains("api.deezer.com") { Err(WebError::Busy(120)) } else { Ok(nothing_on_wikipedia()) });
        backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!(web.calls_to("api.deezer.com"), 1);
        let row = lookup_row(&db.lock().unwrap(), "artist:coldplay").unwrap().unwrap();
        assert_eq!(row.outcome, "failed");
        // Asked by hand while Deezer is still pausing us: Deezer is not called.
        let concert = category_id(&db.lock().unwrap(), "Concert");
        suggest_with(&db, &web, &input(None, "Coldplay", Some(concert), "refresh"), now()).unwrap();
        assert_eq!(web.calls_to("api.deezer.com"), 1);
    }

    #[test]
    fn a_match_finds_its_home_side_and_a_national_team_through_its_sport() {
        let db = db();
        let (club_match, national) = {
            let c = db.lock().unwrap();
            (add_event(&c, "Real Madrid vs Barcelona", Some("Sports"), "2026-11-01"), add_event(&c, "Slovakia vs Germany", Some("Football"), "2026-11-02"))
        };
        let web = FakeWeb::new(|url| {
            if url.contains("gsrsearch=Real%20Madrid&") {
                Ok(wiki_answer(vec![
                    wiki_page(1, 26_412, "Real Madrid CF", "Spanish association football club", Some(&thumb("Real_Madrid_CF.svg")), "Q8682"),
                    wiki_page(2, 1_234, "Real Madrid Baloncesto", "Professional basketball team in Madrid, Spain", Some(&thumb("RMB.svg")), "Q216063"),
                ]))
            } else if url.contains("gsrsearch=Slovakia&") {
                Ok(wiki_answer(vec![wiki_page(1, 26_830, "Slovakia", "Country in Central Europe", Some(&thumb("Flag_of_Slovakia.svg")), "Q214")]))
            } else if url.contains("gsrsearch=Slovakia%20football&") {
                Ok(wiki_answer(vec![wiki_page(
                    1,
                    1_573_404,
                    "Slovakia national football team",
                    "Men's national association football team representing Slovakia",
                    Some(&thumb("Slovakia_FA.svg")),
                    "Q172225",
                )]))
            } else {
                Ok(nothing_on_wikipedia())
            }
        });
        let r = backfill_with(&db, &web, 5, now()).unwrap();
        assert_eq!(r.found, 2, "{r:?}");
        let map = map_of(&db);
        assert_eq!(map[&club_match].key, "a:team:real madrid");
        assert_eq!(map[&national].key, "a:team:slovakia");
        assert_eq!(web.calls_to("api.deezer.com"), 0, "a match is never looked for among musicians");
    }

    #[test]
    fn one_search_at_a_time_for_one_subject() {
        let db = db();
        let concert = {
            let c = db.lock().unwrap();
            add_event(&c, "Coldplay", Some("Concert"), "2026-11-01");
            category_id(&c, "Concert")
        };
        let mut web = music(coldplay());
        web.delay = Duration::from_millis(300);
        std::thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| suggest_with(&db, &web, &input(None, "Coldplay", Some(concert), "search"), now()).unwrap());
            }
        });
        assert_eq!(web.json_calls(), 1, "four at once, one request");
    }

    #[test]
    fn switched_off_means_nothing_is_asked_on_its_own() {
        let db = db();
        let concert = {
            let c = db.lock().unwrap();
            add_event(&c, "Coldplay", Some("Concert"), "2026-11-01");
            set_setting(&c, ARTWORK_AUTO_KEY, "0").unwrap();
            category_id(&c, "Concert")
        };
        let web = music(coldplay());
        assert!(backfill_with(&db, &web, 5, now()).unwrap().paused);
        let s = suggest_with(&db, &web, &input(None, "Coldplay", Some(concert), "search"), now()).unwrap();
        assert_eq!(s.outcome, "off");
        assert_eq!(web.json_calls(), 0);
        // Asked for by hand, it is found.
        let s = suggest_with(&db, &web, &input(None, "Coldplay", Some(concert), "refresh"), now()).unwrap();
        assert_eq!(s.outcome, "match");
    }

    #[test]
    fn a_damaged_picture_shows_the_placeholder_and_is_looked_for_again() {
        let db = db();
        let id = add_event(&db.lock().unwrap(), "Coldplay", Some("Concert"), "2026-11-01");
        backfill_with(&db, &music(coldplay()), 5, now()).unwrap();
        let c = db.lock().unwrap();
        c.execute("UPDATE artwork_lookups SET image = 'data:image/png;base64,bm90IGEgcGljdHVyZQ=='", []).unwrap();
        assert!(artwork_images(&c, &["a:artist:coldplay".to_string()]).unwrap().is_empty());
        assert_eq!(lookup_row(&c, "artist:coldplay").unwrap().unwrap().outcome, "failed");
        drop(c);
        assert!(!map_of(&db).contains_key(&id));
        assert_eq!(backfill_with(&db, &music(coldplay()), 5, now()).unwrap().found, 1);
        // A damaged picture chosen by hand: the placeholder, never an error.
        let c = db.lock().unwrap();
        decide(&c, &choose("name:coldplay", "artist:coldplay", Some(png_url()), "set")).unwrap();
        c.execute("UPDATE event_artworks SET image = 'data:image/png;base64,AAAA'", []).unwrap();
        assert!(artwork_images(&c, &["d:name:coldplay".to_string()]).unwrap().is_empty());
    }

    #[test]
    fn the_choices_and_the_answers_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("tiqr_artwork_{}_{}", std::process::id(), SCOPES.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("restart.sqlite3");
        let (a, b) = {
            let conn = crate::db::open_connection(&path).unwrap();
            crate::db::run_migrations(&conn).unwrap();
            let a = add_event(&conn, "Coldplay", Some("Concert"), "2026-11-01");
            let b = add_event(&conn, "Muse", Some("Concert"), "2026-11-02");
            decide(&conn, &choose("name:muse", "artist:muse", Some(png_url()), "set")).unwrap();
            let db = Mutex::new(conn);
            backfill_with(&db, &music(coldplay()), 5, now()).unwrap();
            (a, b)
        };
        // Started again: everything is still there, and nothing is asked.
        let conn = crate::db::open_connection(&path).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let db = Mutex::new(conn);
        let map = map_of(&db);
        assert_eq!(map[&a].key, "a:artist:coldplay");
        assert_eq!(map[&b].key, "d:name:muse");
        let offline = FakeWeb::offline();
        assert_eq!(backfill_with(&db, &offline, 5, now()).unwrap().searched, 0);
        assert_eq!(offline.json_calls(), 0);
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_answers_kept_here_stay_bounded() {
        let conn = test_conn();
        conn.execute_batch("BEGIN").unwrap();
        for i in 0..(MAX_LOOKUPS + 40) {
            conn.execute(
                "INSERT INTO artwork_lookups (query_key, outcome, checked_at) VALUES (?1, 'none', ?2)",
                params![format!("artist:n{i}"), format!("2026-01-01T00:00:{:02}.000Z", i % 60)],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
        store_lookup(&conn, "artist:one more", &Ok(Vec::new()), now()).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM artwork_lookups", [], |r| r.get(0)).unwrap();
        assert_eq!(n, MAX_LOOKUPS);
        let newest: i64 = conn.query_row("SELECT COUNT(*) FROM artwork_lookups WHERE query_key = 'artist:one more'", [], |r| r.get(0)).unwrap();
        assert_eq!(newest, 1);
    }

    #[test]
    fn a_search_answer_never_marks_the_database_as_changed_for_the_backup() {
        assert!(crate::db::counts_as_user_change("main", "event_artworks"));
        assert!(!crate::db::counts_as_user_change("main", "artwork_lookups"));
    }

    #[test]
    fn thousands_of_events_are_mapped_quickly() {
        let db = db();
        {
            let c = db.lock().unwrap();
            c.execute_batch("BEGIN").unwrap();
            for i in 0..3000 {
                add_event(&c, &format!("Artist {}", i % 300), Some("Concert"), "2026-11-01");
            }
            for i in 0..100 {
                decide(&c, &choose(&format!("name:artist {i}"), &format!("artist:artist {i}"), Some(png_url()), "set")).unwrap();
            }
            c.execute_batch("COMMIT").unwrap();
        }
        let started = Instant::now();
        let map = map_of(&db);
        let took = started.elapsed();
        eprintln!("[perf] artwork map, 3000 events, 100 choices: {took:?}");
        assert_eq!(map.len(), 1000);
        assert!(took < Duration::from_secs(5), "{took:?}");
    }
}
