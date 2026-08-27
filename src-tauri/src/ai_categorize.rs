// TIQR Manager - automatic event category detection (2.0.63)
//
// marko's own request, several rounds of AskUserQuestion to pin down exactly
// what he meant by it:
//
//   1. Sport stays a single "Sports" category (never Football/Tennis/...
//      auto-created) - simplest, matches the 6 categories 012_event_
//      categories.sql already seeded.
//   2. A bare name with no textual signal at all (his own example: "Celine
//      Dion") must NOT be left uncategorized as a cop-out, and must NOT be
//      blindly defaulted to "Concert" as a guess either - his words: "musi
//      vediet zistit o co ide" (it has to be able to figure out what it
//      is).
//   3. The same goes for a "Team A - Team B" style name (his own example:
//      "Liverpool - Nottingham Forest") - his words: "musi to vediet
//      rozoznat" (it has to be able to recognize/tell what it is), i.e. not
//      a blind "any hyphen between two capitalized words = Sports" pattern
//      match, which would also misfire on a two-headliner concert titled
//      the same way.
//   4. Existing uncategorized events get a one-time retroactive pass too
//      (see commands::events::detect_event_categories), not just new
//      events going forward.
//
// Point 2 and 3 both boil down to the same thing: correctly identifying an
// arbitrary team/artist/performer BY NAME is real-world knowledge, not
// something a hand-maintained keyword list can ever fully cover (there is
// no realistic offline list of every football club and touring musician on
// earth). A plain keyword/pattern rule can only ever catch the minority of
// event names that already contain an explicit, unambiguous signal in the
// text itself. So this module is deliberately a HYBRID, exactly as marko
// confirmed when asked directly:
//
//   - `free_rule_category_name` runs first, for free, and only ever fires
//     on a small, deliberately conservative keyword list for the four
//     categories that really do have safe, distinctive textual signals
//     (Motorsport/Festival/Theatre-Musical/Comedy). Notably, Sports is NOT
//     given a free keyword/pattern rule here - a generic "Team A - Team B"
//     match was exactly what marko flagged as needing real recognition
//     rather than a blind pattern (see point 3 above), so every sports
//     matchup is deliberately left to the AI path below instead.
//   - When the free rules find nothing, `detect_category_for_event_name`
//     falls back to actually asking an AI model (Claude Haiku, chosen for
//     being Anthropic's fastest/cheapest tier - a single short classification
//     call costs a small fraction of a cent) which of the app's *existing*
//     category names the event/team/artist actually belongs to, or "none"
//     if it doesn't recognize it. This is what gives points 2 and 3 real
//     "figure out what it is" behavior instead of a guess.
//
// Every failure mode - no rule match, no API key embedded in this build, no
// network, a non-2xx response, a reply that names a category that doesn't
// actually exist - collapses to the same `None`: "could not confidently
// categorize this," never a wrong guess, and never something that blocks
// whatever the caller (a live sheet sync, or the retroactive button) is
// doing. That soft-fail behavior is baked into `detect_category_for_event_
// name`'s own signature (it returns `Option`, not `AppResult`) specifically
// so a caller cannot accidentally treat a failed classification as a hard
// error.
//
// The API key itself follows the *exact* precedent build.rs already set for
// GOOGLE_SERVICE_ACCOUNT_JSON: embedded at build time from a GitHub Actions
// repository secret (ANTHROPIC_API_KEY), never a tracked file in this
// public repo, and never something the app asks marko to type into
// Settings. A plain local `cargo build`/`cargo test` never has it set - see
// `embedded_anthropic_api_key`'s own doc comment - and this feature is
// designed so that being unconfigured is a completely normal, silent state:
// the free rules still work, and everything else is simply left
// uncategorized exactly like every event is today.

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Mirrors `google_sheets::EMBEDDED_SERVICE_ACCOUNT_JSON`/`embedded_service_
/// account` exactly - see build.rs for the embed step and this module's own
/// top doc comment for why. Empty (the normal state for every local build,
/// and for a real release build until marko adds the GitHub Actions secret
/// himself) means "this feature's AI half isn't configured," not an error.
const EMBEDDED_ANTHROPIC_API_KEY: &str =
    include_str!(concat!(env!("OUT_DIR"), "/anthropic_api_key.txt"));

/// `None` on a plain local build/test, or a real release built before marko
/// adds the `ANTHROPIC_API_KEY` GitHub Actions secret. Never panics either
/// way - see this module's top doc comment.
pub fn embedded_anthropic_api_key() -> Option<String> {
    let key = EMBEDDED_ANTHROPIC_API_KEY.trim();
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
// Anthropic's fastest/cheapest model as of 2.0.63 - a good fit for a single
// short classification reply. If Anthropic ever retires this exact id, this
// call starts failing closed (AppError::External, swallowed by `detect_
// category_for_event_name` into a plain `None`) rather than doing anything
// destructive - see docs.claude.com/en/docs/about-claude/models for the
// current model list if this ever needs bumping.
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5-20251001";

// 2.0.64: marko asked for this to be "co najlacnejsie" (as cheap as
// possible) AND "bezchybne" (flawless) - the two constants below are the
// cheap-side and reliability-side knobs, and they don't actually trade off
// against each other the way they might look like they do:
//
// - ANTHROPIC_MAX_TOKENS is a CEILING, not something billed up front -
//   Anthropic only charges for tokens the model actually generates (its own
//   documented "failed requests aren't charged" pricing model extends the
//   same way to unused max_tokens headroom), and a correctly-behaving reply
//   here is always just a short category name (a handful of tokens) because
//   build_prompt already tells the model to answer with nothing else. So
//   raising this doesn't cost more in the normal case - it only exists to
//   stop a pathological runaway reply from costing more AND, just as
//   important for "bezchybne," to never truncate a genuinely long custom
//   category name marko might create by hand (a low ceiling here would
//   silently cut such a name short, which `parse_ai_category_reply` would
//   then correctly - but uselessly - reject as "not a real category").
// - ANTHROPIC_TIMEOUT_SECS bounds how long ONE call can block a sync. The
//   Google Sheets calls in google_sheets.rs deliberately have no timeout at
//   all (see that module) - but this call sits inside a per-new-event loop
//   (resolve_or_create_event, and every event `detect_event_categories`
//   scans), so a single hung request there risks hanging the entire
//   sync/scan, not just one API call. 20s is generous for a reply this
//   short while still guaranteeing the app can never freeze on this
//   indefinitely.
const ANTHROPIC_MAX_TOKENS: u32 = 40;
const ANTHROPIC_TIMEOUT_SECS: u64 = 20;

/// The result of one successful categorization, however it was reached.
/// Carries both the id and the name (rather than making every caller look
/// the name back up) because `events.category` is a denormalized text
/// mirror of `events.category_id`'s name - see migrations/012_event_
/// categories.sql's doc comment for why that mirror must never drift - so
/// every write site needs both values at once, not just the id.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryMatch {
    pub id: i64,
    pub name: String,
    /// True when this came from the AI fallback rather than a free keyword
    /// rule - `commands::events::detect_event_categories` uses this to
    /// report "resolved by a rule" separately from "resolved by AI" in its
    /// summary, without re-deriving it.
    pub via_ai: bool,
}

/// Zero-cost, deliberately conservative keyword rules - see this module's
/// top doc comment for why exactly these four categories, and not a
/// generic Sports rule, are the only ones handled for free. A case-
/// insensitive substring match on the raw event name; every keyword here
/// was chosen for being very unlikely to appear in an unrelated event name
/// (unlike, say, a bare hyphen or the word "cup," which could just as
/// easily be part of a concert tour's own title).
fn free_rule_category_name(event_name: &str) -> Option<&'static str> {
    let lower = event_name.to_lowercase();
    const MOTORSPORT: &[&str] =
        &["grand prix", "formula 1", "formula1", "moto gp", "motogp", "nascar", "rally"];
    const FESTIVAL: &[&str] = &["festival"];
    const THEATRE: &[&str] = &["musical", "divadlo", "theatre", "theater"];
    const COMEDY: &[&str] = &["comedy", "stand-up", "stand up"];

    let any = |kws: &[&str]| kws.iter().any(|k| lower.contains(k));
    if any(MOTORSPORT) {
        Some("Motorsport")
    } else if any(FESTIVAL) {
        Some("Festival")
    } else if any(THEATRE) {
        Some("Theatre / Musical")
    } else if any(COMEDY) {
        Some("Comedy")
    } else {
        None
    }
}

/// 2.0.64: trimmed to the shortest wording that still reliably gets a clean,
/// correctly-formatted answer (marko's "co najlacnejsie" - fewer input
/// tokens is fewer cents, however small the difference) - see this file's
/// prompt-related tests for what still has to keep working after any future
/// trim: a real match, a hallucination-proof "none," and no stray
/// punctuation wrapping the answer.
fn build_prompt(event_name: &str, candidates: &[String]) -> String {
    let list = candidates.iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n");
    format!(
        "Ticket resale event: \"{event_name}\"\n\
         Pick the one category below that fits, using what you actually know about this team, artist, \
         or event. Reply with ONLY that exact category name, or \"none\" if you don't recognize it or \
         none fit - no other words.\n{list}"
    )
}

/// Validates the model's raw reply against the exact candidate list handed
/// to it, so a hallucinated or misspelled category name can never reach the
/// database - only ever one of the names actually offered, matched case-
/// insensitively with surrounding whitespace/quotes/a trailing period
/// stripped (the model occasionally adds one of those despite being asked
/// not to). Pure and network-free on purpose, so it can be unit tested
/// directly without a live API key - see this file's tests module.
fn parse_ai_category_reply(raw: &str, candidates: &[String]) -> Option<String> {
    let cleaned = raw.trim().trim_matches(|c: char| c == '"' || c == '\'' || c == '.');
    if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("none") {
        return None;
    }
    candidates.iter().find(|c| c.eq_ignore_ascii_case(cleaned)).cloned()
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(Debug, Default, Deserialize)]
struct AnthropicContentBlock {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
}

/// 2.0.64: true for a failure that's worth paying for a second attempt at -
/// a momentary rate limit or a transient server hiccup - false for anything
/// that would fail the exact same way again (a bad key, a malformed
/// request, ...), where retrying would just spend money twice for one
/// answer. Pure and easy to get exactly right, which matters here: this is
/// the one piece of "retry on transient failure" that's actually worth unit
/// testing directly (the HTTP call itself stays in the untested-at-that-
/// layer bucket, same convention as google_sheets.rs's own calls).
fn is_retriable_anthropic_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

/// One real attempt at the call - `Ok` with the raw response body on a 2xx,
/// or `Err((status, body-or-transport-error-text))` otherwise, so the
/// caller can decide whether this particular failure is worth retrying
/// (`is_retriable_anthropic_status`) before it ever becomes the `AppError`
/// the rest of the app sees. `status` is `None` only when the request never
/// even reached Anthropic at all (DNS/connection/timeout) - `is_retriable_
/// anthropic_status` never gets to weigh in on that case either way, see
/// `ai_classify_category_name` below.
fn send_anthropic_request(api_key: &str, prompt: &str) -> Result<String, (Option<reqwest::StatusCode>, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(ANTHROPIC_TIMEOUT_SECS))
        .build()
        .map_err(|e| (None, format!("could not build the HTTP client: {e}")))?;
    let body = AnthropicRequest {
        model: ANTHROPIC_MODEL,
        max_tokens: ANTHROPIC_MAX_TOKENS,
        temperature: 0.0,
        messages: vec![AnthropicMessage { role: "user", content: prompt }],
    };
    let resp = client
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| (None, format!("could not reach Anthropic: {e}")))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| (Some(status), format!("could not read Anthropic's response: {e}")))?;
    if !status.is_success() {
        return Err((Some(status), text));
    }
    Ok(text)
}

/// The one network call this module makes. Thin and deliberately untested
/// at this exact layer - same convention google_sheets.rs already follows
/// for its own HTTP calls (see e.g. `fetch_access_token`'s doc comment) -
/// all the actual decision logic lives in the pure functions around it
/// (`free_rule_category_name`, `parse_ai_category_reply`, `is_retriable_
/// anthropic_status`), which are what this file's tests exercise directly.
///
/// 2.0.64: retries EXACTLY once, after a short pause, and only when `send_
/// anthropic_request` failed with a status `is_retriable_anthropic_status`
/// agrees is worth a second try - marko's own request ("chcem aby to bolo
/// bezchybne") is exactly what this guards: a single momentary rate limit
/// or server hiccup must not silently cost an otherwise-resolvable
/// categorization. A transport-level failure (no status at all - DNS,
/// connection refused, this app's own 20s timeout) is treated the same as
/// a permanent failure and never retried, since it's unlikely to have
/// cleared in under a second and the caller (`detect_category_for_event_
/// name`) already turns any remaining failure into a plain, harmless
/// `None` either way - never a wrong guess, never a crash.
fn ai_classify_category_name(
    api_key: &str,
    event_name: &str,
    candidates: &[String],
) -> AppResult<Option<String>> {
    let prompt = build_prompt(event_name, candidates);
    let mut attempt = send_anthropic_request(api_key, &prompt);
    if let Err((Some(status), _)) = attempt {
        if is_retriable_anthropic_status(status) {
            std::thread::sleep(std::time::Duration::from_millis(750));
            attempt = send_anthropic_request(api_key, &prompt);
        }
    }
    let text = attempt.map_err(|(status, body)| match status {
        Some(s) => AppError::External(format!("Anthropic rejected the classification request ({s}): {body}")),
        None => AppError::External(format!("could not reach Anthropic to classify '{event_name}': {body}")),
    })?;
    let parsed: AnthropicResponse = serde_json::from_str(&text).map_err(|e| {
        AppError::External(format!("unexpected response from Anthropic: {e} (body: {text})"))
    })?;
    let raw_reply = parsed.content.first().map(|b| b.text.as_str()).unwrap_or("");
    Ok(parse_ai_category_reply(raw_reply, candidates))
}

/// The single entry point both the automatic path (a brand-new event
/// created during a live sheet sync, see commands::orders_sheet_sync::
/// resolve_or_create_event) and the manual "Detect categories" button
/// (commands::events::detect_event_categories) call. See this module's top
/// doc comment for the full free-rules-then-AI design and why. Returns
/// `None` - not an `Err` - for every failure mode, by design: a caller can
/// treat this exactly like "no opinion," never like something that needs
/// to abort or be surfaced as a hard error.
pub fn detect_category_for_event_name(conn: &Connection, event_name: &str) -> Option<CategoryMatch> {
    let trimmed = event_name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lookup_id = |name: &str| -> Option<i64> {
        conn.query_row("SELECT id FROM event_categories WHERE LOWER(name) = LOWER(?1)", [name], |r| r.get(0))
            .ok()
    };

    if let Some(rule_name) = free_rule_category_name(trimmed) {
        if let Some(id) = lookup_id(rule_name) {
            return Some(CategoryMatch { id, name: rule_name.to_string(), via_ai: false });
        }
        // The rule pointed at a name (e.g. "Motorsport") that doesn't exist
        // in *this* database - marko renamed or deleted it - so fall
        // through to the AI path below rather than giving up; it's handed
        // the real, live category list either way.
    }

    let api_key = embedded_anthropic_api_key()?;
    let mut stmt = conn.prepare("SELECT name FROM event_categories ORDER BY name").ok()?;
    let candidates: Vec<String> =
        stmt.query_map([], |r| r.get::<_, String>(0)).ok()?.filter_map(|r| r.ok()).collect();
    if candidates.is_empty() {
        return None;
    }
    let ai_name = ai_classify_category_name(&api_key, trimmed, &candidates).ok().flatten()?;
    let id = lookup_id(&ai_name)?;
    Some(CategoryMatch { id, name: ai_name, via_ai: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(vals: &[&str]) -> Vec<String> {
        vals.iter().map(|s| s.to_string()).collect()
    }

    // -- free_rule_category_name ---------------------------------------

    #[test]
    fn recognizes_motorsport_keywords_case_insensitively() {
        assert_eq!(free_rule_category_name("Monaco Grand Prix 2026"), Some("Motorsport"));
        assert_eq!(free_rule_category_name("FORMULA 1 - Spa"), Some("Motorsport"));
        assert_eq!(free_rule_category_name("MotoGP Brno"), Some("Motorsport"));
        assert_eq!(free_rule_category_name("NASCAR Cup Series"), Some("Motorsport"));
        assert_eq!(free_rule_category_name("Monte Carlo Rally"), Some("Motorsport"));
    }

    #[test]
    fn recognizes_festival_theatre_and_comedy_keywords() {
        assert_eq!(free_rule_category_name("Reading Festival"), Some("Festival"));
        assert_eq!(free_rule_category_name("The Lion King - The Musical"), Some("Theatre / Musical"));
        assert_eq!(free_rule_category_name("Národné divadlo - Labutie jazero"), Some("Theatre / Musical"));
        assert_eq!(free_rule_category_name("Live Comedy Night"), Some("Comedy"));
        assert_eq!(free_rule_category_name("John Doe: Stand-Up Tour"), Some("Comedy"));
    }

    #[test]
    fn a_bare_name_with_no_keyword_signal_matches_no_free_rule() {
        // marko's own example - this must fall through to the AI path
        // rather than a free rule guessing at it.
        assert_eq!(free_rule_category_name("Celine Dion"), None);
    }

    #[test]
    fn a_team_vs_team_style_name_matches_no_free_rule() {
        // marko's own example, and his own explicit answer ("musi to vediet
        // rozoznat") - a bare "A - B" shape must NOT be a free/blind
        // pattern match for Sports, since the exact same shape could be a
        // two-headliner concert. This is deliberately left to the AI path.
        assert_eq!(free_rule_category_name("Liverpool - Nottingham Forest"), None);
    }

    // -- parse_ai_category_reply ----------------------------------------

    #[test]
    fn exact_and_case_insensitive_replies_match_a_real_candidate() {
        let cats = names(&["Concert", "Sports", "Festival"]);
        assert_eq!(parse_ai_category_reply("Sports", &cats), Some("Sports".to_string()));
        assert_eq!(parse_ai_category_reply("sports", &cats), Some("Sports".to_string()));
        assert_eq!(parse_ai_category_reply("  Concert  ", &cats), Some("Concert".to_string()));
    }

    #[test]
    fn strips_a_trailing_period_or_surrounding_quotes() {
        let cats = names(&["Concert", "Sports"]);
        assert_eq!(parse_ai_category_reply("Concert.", &cats), Some("Concert".to_string()));
        assert_eq!(parse_ai_category_reply("\"Sports\"", &cats), Some("Sports".to_string()));
    }

    #[test]
    fn the_word_none_and_an_empty_reply_both_mean_no_match() {
        let cats = names(&["Concert", "Sports"]);
        assert_eq!(parse_ai_category_reply("none", &cats), None);
        assert_eq!(parse_ai_category_reply("None", &cats), None);
        assert_eq!(parse_ai_category_reply("", &cats), None);
        assert_eq!(parse_ai_category_reply("   ", &cats), None);
    }

    #[test]
    fn a_hallucinated_name_not_in_the_candidate_list_is_rejected() {
        // The model must never be able to write a category that doesn't
        // actually exist in *this* database, however confidently it answers.
        let cats = names(&["Concert", "Sports"]);
        assert_eq!(parse_ai_category_reply("Football", &cats), None);
        assert_eq!(parse_ai_category_reply("Theatre / Musical", &cats), None);
    }

    // -- is_retriable_anthropic_status (2.0.64) ----------------------------

    #[test]
    fn a_rate_limit_or_server_hiccup_is_worth_retrying() {
        assert!(is_retriable_anthropic_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retriable_anthropic_status(reqwest::StatusCode::REQUEST_TIMEOUT));
        assert!(is_retriable_anthropic_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retriable_anthropic_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
    }

    #[test]
    fn a_bad_key_or_bad_request_is_never_retried() {
        // These would fail exactly the same way a second time - retrying
        // would only spend money twice for the one guaranteed-same answer.
        assert!(!is_retriable_anthropic_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retriable_anthropic_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retriable_anthropic_status(reqwest::StatusCode::FORBIDDEN));
        assert!(!is_retriable_anthropic_status(reqwest::StatusCode::NOT_FOUND));
    }

    // -- embedded_anthropic_api_key ---------------------------------------

    #[test]
    fn embedded_anthropic_api_key_is_none_on_a_plain_local_build() {
        // This test suite never has ANTHROPIC_API_KEY set (build.rs falls
        // back to an empty embed, mirroring GOOGLE_SERVICE_ACCOUNT_JSON's
        // own convention exactly), so this must hold in exactly the
        // environment this test actually runs in.
        assert!(
            embedded_anthropic_api_key().is_none(),
            "a local cargo test/build must never have a real key embedded"
        );
    }

    // -- detect_category_for_event_name ------------------------------------
    //
    // These use the crate's own `db::test_conn()` (in-memory, fully migrated
    // via the real `run_migrations`) rather than a hand-rolled schema, so
    // they exercise the actual 6 categories 012_event_categories.sql really
    // seeds - same convention every other command module's tests already
    // follow (see e.g. commands::orders_sheet_sync's own test module).

    use crate::db::test_conn;

    #[test]
    fn a_free_rule_match_resolves_without_needing_any_api_key() {
        let conn = test_conn();
        let m = detect_category_for_event_name(&conn, "Spa-Francorchamps Grand Prix").unwrap();
        assert_eq!(m.name, "Motorsport");
        assert!(!m.via_ai, "a keyword-rule match must never be reported as coming from AI");
    }

    #[test]
    fn a_bare_name_is_left_uncategorized_when_no_api_key_is_embedded() {
        // In this test environment `embedded_anthropic_api_key()` is always
        // None (see the test above), so both of marko's own hard examples
        // must safely resolve to "no opinion" rather than panicking or
        // guessing - exactly like every event does today.
        let conn = test_conn();
        assert!(detect_category_for_event_name(&conn, "Celine Dion").is_none());
        assert!(detect_category_for_event_name(&conn, "Liverpool - Nottingham Forest").is_none());
    }

    #[test]
    fn a_blank_name_is_never_classified() {
        let conn = test_conn();
        assert!(detect_category_for_event_name(&conn, "").is_none());
        assert!(detect_category_for_event_name(&conn, "   ").is_none());
    }

    #[test]
    fn a_free_rule_pointing_at_a_category_name_that_does_not_exist_here_does_not_panic() {
        // marko renamed/deleted "Motorsport" in this hypothetical database -
        // resolving must fall through cleanly (to the AI path, which then
        // also finds nothing without a key) rather than erroring.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE event_categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
             INSERT INTO event_categories (id, name) VALUES (1,'Concert');",
        )
        .unwrap();
        assert!(detect_category_for_event_name(&conn, "Monaco Grand Prix").is_none());
    }
}
