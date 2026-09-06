// TIQR Manager - AI Import Assistant (2.7.0)
//
// marko's own request: let him drop/paste a screenshot into the New Event,
// New Order and New Sale forms and have Claude read the structured data off
// it, so he doesn't retype a marketplace confirmation by hand.
//
// The one rule this whole module exists to enforce, in his own words: "AI
// NIKDY nesmie priamo vytvoriť alebo meniť databázový záznam." So:
//
//   IMAGE -> CLAUDE -> STRUCTURED RESULT -> PREVIEW -> USER CONFIRM
//         -> the EXISTING create form -> the EXISTING create command -> DB
//
// This module is the first two arrows and nothing else. It is deliberately
// **stateless and database-free**: `analyze_import_image` takes no
// `AppState`, opens no `Connection`, and has no write path of any kind. It
// cannot create an event, an order, a sale or a ticket even if it wanted
// to - the only thing it can do is return a struct to the frontend, which
// then pre-fills form fields the user still has to look at and submit
// through the app's existing create flow. Keep it that way: if a future
// change needs this module to touch the database, that is a redesign to
// discuss with marko first, not an incremental edit.
//
// Relationship to `crate::ai_categorize` (2.0.63, the other - and until now
// only - Anthropic caller in this app): that module owns the credential and
// the retry policy, and this one reuses both rather than standing up a
// second, drifting copy:
//
//   - `ai_categorize::embedded_anthropic_api_key()` - the SAME build-time
//     embedded `ANTHROPIC_API_KEY` (see build.rs), never a key typed into
//     Settings, never a key that reaches the frontend. That is what
//     satisfies marko's own "NEVKLADAJ API key do UI source" requirement:
//     the key never leaves the Rust side of the IPC boundary.
//   - `ai_categorize::is_retriable_anthropic_status()` - the same
//     retry-once-on-a-transient-status policy, for the same reason (a
//     momentary 429 shouldn't cost marko a re-upload), and the same refusal
//     to retry a permanent failure twice for one answer.
//
// The one deliberate difference from `ai_categorize`: THIS module returns
// real `AppError`s instead of soft-failing to `None`. Categorization is a
// background nicety where "no opinion" is a fine outcome; here marko
// explicitly clicked "analyze" and is waiting, so a failure has to be
// visible to him ("AI analysis failed. Try again.") rather than silently
// producing an empty result that looks like "the screenshot had nothing in
// it."
//
// Cost control (marko's section 14, "Toto je DÔLEŽITÉ") is enforced on BOTH
// sides and it matters that it is not only the frontend's job:
//   - Frontend (`lib/aiImport.ts`): one analysis per explicit user action,
//     never on a timer, never on mount, and a per-session fingerprint cache
//     so re-dropping the same image (or editing extracted fields) never
//     spends a second request.
//   - Here: exactly ONE `messages` call per invocation, at most one retry
//     and only on a transient status, a hard `max_tokens` ceiling, and no
//     internal loop of any kind. There is no code path in this file that
//     can call Anthropic twice for two different prompts.

use crate::ai_categorize::{embedded_anthropic_api_key, is_retriable_anthropic_status};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Reading small, dense text off a marketplace screenshot correctly is the
/// entire value of this feature - a misread seat range or price is worse
/// than no extraction at all, because marko might not notice it in the
/// review step. So this deliberately does NOT reuse `ai_categorize`'s
/// Haiku: that model is the right call for a one-word classification, not
/// for pulling a dozen exact fields out of an image.
///
/// If marko ever wants to trade accuracy for cost, this constant and
/// `ANTHROPIC_EFFORT` below are the only two things to change - the rest of
/// the module is model-agnostic.
const ANTHROPIC_MODEL: &str = "claude-opus-5";

/// `low` | `medium` | `high` | `xhigh` | `max`. Extraction from one image is
/// a bounded, well-specified task rather than open-ended reasoning, so this
/// sits below the API's own `high` default - that is the cost lever for this
/// feature, chosen ahead of dropping to a weaker model (see
/// `ANTHROPIC_MODEL`).
const ANTHROPIC_EFFORT: &str = "medium";

/// Generous enough for the worst realistic case - a multi-group order
/// screenshot with a dozen seats - without leaving room for a runaway
/// reply. Same reasoning as `ai_categorize`'s own ceiling: a ceiling is not
/// billed up front, only what the model actually generates is, so this
/// exists to bound a pathological reply and to never truncate a legitimate
/// one. A truncated reply is detected explicitly below (`stop_reason ==
/// "max_tokens"`) rather than silently parsed as partial data.
const ANTHROPIC_MAX_TOKENS: u32 = 8_000;

/// Longer than `ai_categorize`'s 20s: that call is a handful of output
/// tokens, this one reads an image and writes a structured object. Still
/// bounded, because marko is sitting in front of a spinner while it runs.
const ANTHROPIC_TIMEOUT_SECS: u64 = 90;

/// Anthropic's own per-image ceiling. Checked here as well as in the
/// frontend (which downscales before it ever gets this far) so a hand-built
/// IPC call can't get past it either - the frontend is a convenience, not
/// the enforcement point.
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// marko's section 11. GIF is intentionally absent even though Anthropic
/// accepts it: he asked for PNG/JPG/WebP, and every extra accepted type is
/// one more thing to have an opinion about.
const ALLOWED_MEDIA_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

// ---------------------------------------------------------------------------
// What this module returns
// ---------------------------------------------------------------------------

/// One extracted field, in marko's own requested shape:
/// `{ "field": "section", "value": "402", "confidence": "high" }`.
///
/// `value` is ALWAYS a string or null, never a number, date or enum, and
/// that is deliberate rather than lazy: every create form in this app holds
/// its fields as strings too (`OrderFormModal`'s `quantity`/`unitPrice`/
/// `seatsRaw`, `EventFormModal`'s `eventDate`, ...), so a string maps
/// straight into an input with no parsing layer in between - and, more
/// importantly, no second place where a value could be silently coerced,
/// rounded or reinterpreted before the app's own existing validation sees
/// it at save time. Claude is not a source of truth here; the existing
/// validation is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiImportField {
    /// One of the field names this import kind asked for - see
    /// `field_names_for_kind`. A name outside that list is dropped by
    /// `sanitize_result` rather than shown.
    pub field: String,
    /// `None` whenever the value is not actually visible in the image.
    /// marko's own rule: "Ak údaj nie je na obrázku: value = null" - never
    /// an inferred, defaulted or "probably" value.
    pub value: Option<String>,
    /// `high` | `medium` | `low`. Anything else is normalised to `low` by
    /// `normalize_confidence`, so the frontend only ever has 3 cases to
    /// render and a malformed reply degrades to "treat this with
    /// suspicion," never to a missing badge.
    pub confidence: String,
}

/// One block of tickets that share a tier/section/row/price - marko's own
/// example: 4 tickets in Tier 100 / Section 102 / Row 14 / seats 21-24 at
/// EUR 180 each, alongside a separate group of 2 in Section 205.
///
/// These are returned SEPARATELY and are never merged ("Nechcem zlievať
/// rôzne groups do jednej"), because merging them would invent a
/// section/row/price that no ticket actually has. Note what this means
/// downstream: this app's `OrderInput` carries ONE section/row/tier/price
/// for the whole order, so two groups are two orders, and the frontend
/// fills one group at a time rather than inventing a multi-group order
/// shape the backend has never had - see `AiImportPanel.tsx`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiImportTicketGroup {
    pub quantity: Option<String>,
    pub tier: Option<String>,
    pub section: Option<String>,
    pub row: Option<String>,
    pub ticket_type: Option<String>,
    /// Per-ticket price, not the group total - the same grain
    /// `OrderFormModal`'s own "Unit price" field uses.
    pub unit_price: Option<String>,
    pub fees: Option<String>,
    /// One label per seat, already expanded ("21", "22", "23", "24" rather
    /// than "21-24"), because that is the shape `OrderInput.seats` wants -
    /// one label per generated ticket. Empty when the image doesn't show
    /// individual seats.
    pub seats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiImportResult {
    /// False when the model judged the image too poor to read reliably.
    /// The frontend shows marko's own wording ("Image quality too low to
    /// reliably extract data.") and offers a retry - it does NOT show
    /// half-read fields, which is the whole point: a low-confidence guess
    /// presented as data is worse than an honest failure.
    pub readable: bool,
    pub fields: Vec<AiImportField>,
    /// Always empty for `event` and `sale` - see `AiImportTicketGroup`.
    pub ticket_groups: Vec<AiImportTicketGroup>,
}

// ---------------------------------------------------------------------------
// Per-kind schema + prompt
// ---------------------------------------------------------------------------

/// The three places this feature is offered from. Kept as a plain `&str`
/// across the IPC boundary (rather than a serde enum) so an unrecognised
/// value is a clean `AppError::Validation` here instead of a deserialisation
/// error the frontend can't render nicely.
fn field_names_for_kind(kind: &str) -> AppResult<&'static [&'static str]> {
    match kind {
        // marko's section 4. `status` only when it is literally printed on
        // the image - see `build_prompt`'s own wording; he was explicit
        // that AI must never decide an event's status by reasoning.
        "event" => Ok(&["name", "eventDate", "venue", "city", "country", "category", "status"]),
        // marko's section 5. The order-level half; everything per-ticket
        // lives in `ticketGroups` instead.
        "order" => Ok(&[
            "eventName",
            "eventDate",
            "venue",
            "orderReference",
            "platform",
            "purchaseDate",
            "totalPrice",
            "currency",
        ]),
        // marko's section 6. Deliberately has no ticket/seat fields even
        // though he listed them: a sale in this app is always recorded
        // AGAINST an existing ticket row (`SaleInput.ticketId`), so the
        // tickets are picked from the database in the form's own first
        // step, never described by a screenshot. Extracting seat text here
        // would produce something with nowhere valid to go.
        "sale" => Ok(&[
            "eventName",
            "orderReference",
            "saleDate",
            "quantity",
            "salePrice",
            "sellingFees",
            "currency",
            "marketplace",
            "buyerReference",
            "paymentStatus",
            "deliveryStatus",
        ]),
        other => Err(AppError::Validation(format!(
            "unknown AI import kind '{other}' - expected event, order or sale"
        ))),
    }
}

/// The strict JSON schema handed to the API's structured-output mode
/// (`output_config.format`), which is what makes "the model returns JSON"
/// a guarantee of the request rather than a hope about the prompt.
///
/// Two constraints of that mode are load-bearing here and easy to break by
/// accident: EVERY object must set `additionalProperties: false` and list
/// every one of its properties in `required`. Nullability therefore has to
/// be expressed as `anyOf: [string, null]` rather than an optional
/// property - "absent" is not available, only "present and null," which is
/// exactly the distinction marko asked for anyway (a field that isn't in
/// the image comes back explicitly null, not missing).
fn build_schema(kind: &str) -> AppResult<serde_json::Value> {
    let field_names = field_names_for_kind(kind)?;
    // A fresh Value per call rather than one reused binding: `json!` takes
    // each expression by value, so a single `let nullable_string = ...`
    // would be moved by its first use.
    let nullable_string = || json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] });
    Ok(json!({
        "type": "object",
        "properties": {
            "readable": { "type": "boolean" },
            "fields": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "field": { "type": "string", "enum": field_names },
                        "value": nullable_string(),
                        "confidence": { "type": "string", "enum": ["high", "medium", "low"] }
                    },
                    "required": ["field", "value", "confidence"],
                    "additionalProperties": false
                }
            },
            "ticketGroups": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "quantity": nullable_string(),
                        "tier": nullable_string(),
                        "section": nullable_string(),
                        "row": nullable_string(),
                        "ticketType": nullable_string(),
                        "unitPrice": nullable_string(),
                        "fees": nullable_string(),
                        "seats": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": [
                        "quantity", "tier", "section", "row",
                        "ticketType", "unitPrice", "fees", "seats"
                    ],
                    "additionalProperties": false
                }
            }
        },
        "required": ["readable", "fields", "ticketGroups"],
        "additionalProperties": false
    }))
}

/// Every "do not invent anything" rule marko wrote, stated to the model
/// once rather than re-derived per field. The schema above guarantees the
/// SHAPE; this guarantees the HONESTY, and the two are not
/// interchangeable - a schema-valid reply full of plausible guesses would
/// pass every check in this file and still be exactly the outcome he asked
/// us to prevent.
fn build_prompt(kind: &str) -> AppResult<String> {
    let field_names = field_names_for_kind(kind)?;
    let list = field_names.join(", ");
    let kind_specific = match kind {
        "event" => {
            "This image should describe ONE event. Set \"status\" only if a status word is \
             literally printed in the image - never decide it yourself from the date or \
             anything else. Return an empty \"ticketGroups\" array."
        }
        "order" => {
            "This image should describe ONE ticket purchase. Put order-level facts in \
             \"fields\" and per-ticket facts in \"ticketGroups\". Return one group per set of \
             tickets that share a tier, section, row and price - never merge two different \
             sections, rows, tiers or prices into one group, and never split one real group in \
             two. Expand a seat range into individual labels (\"21-24\" becomes \"21\", \"22\", \
             \"23\", \"24\"); leave \"seats\" empty if individual seats are not shown. \
             \"unitPrice\" is the price of ONE ticket - if only a total is shown, put it in the \
             \"totalPrice\" field instead and leave \"unitPrice\" null rather than dividing it \
             yourself."
        }
        "sale" => {
            "This image should describe ONE sale. Set \"paymentStatus\" and \"deliveryStatus\" \
             only if they are literally printed in the image. Do not try to identify which \
             specific tickets were sold - that is chosen elsewhere. Return an empty \
             \"ticketGroups\" array."
        }
        _ => unreachable!("field_names_for_kind already rejected this kind"),
    };
    Ok(format!(
        "You are reading a screenshot for a ticket reseller's records. Extract only what is \
         actually visible in this image.\n\n\
         Rules:\n\
         - Never guess, infer or complete a value that is not shown. If it is not in the image, \
         its value is null.\n\
         - Never invent an event, venue, seat, date or price.\n\
         - Copy values as they appear, without reformatting or converting currencies. For a \
         price, return only the number (\"180.00\"), and put the currency in its own field.\n\
         - Dates are the ONE exception to copying verbatim: return every date as YYYY-MM-DD, \
         because that is the format the app's own date fields take. If the year is not shown \
         anywhere and cannot be read off the image, the date is null - do not assume the current \
         year.\n\
         - confidence: \"high\" = clearly legible and unambiguous, \"medium\" = legible but you \
         had to interpret a label, \"low\" = partly obscured or a guess at the layout.\n\
         - Return one entry in \"fields\" for each of these, in this order, including the ones \
         you had to set to null: {list}\n\
         - Set \"readable\" to false if the image is too blurry, cropped or low-resolution to \
         extract these reliably. When it is false, do not fill fields in with guesses.\n\n\
         {kind_specific}"
    ))
}

// ---------------------------------------------------------------------------
// Input validation (pure - see this file's tests)
// ---------------------------------------------------------------------------

/// Decoded byte length of a standard base64 payload, WITHOUT decoding it.
/// Used only to reject an oversized image early: decoding 5 MB of base64
/// just to measure it, and then throwing it away, is pure waste on the one
/// path where we already know we're going to refuse.
///
/// Returns `None` for a length that cannot be valid base64 at all (`len % 4
/// != 0`), which the caller turns into the same "unsupported image" error
/// as a bad media type - it is not worth a distinct message to marko.
fn base64_decoded_len(data: &str) -> Option<usize> {
    let len = data.len();
    if len == 0 || len % 4 != 0 {
        return None;
    }
    let padding = data.as_bytes()[len - 2..].iter().filter(|b| **b == b'=').count();
    Some(len / 4 * 3 - padding)
}

/// marko's section 11 + 13: reject before spending anything, with the exact
/// short messages he asked for rather than a technical one.
fn validate_image(media_type: &str, image_base64: &str) -> AppResult<()> {
    if !ALLOWED_MEDIA_TYPES.contains(&media_type) {
        return Err(AppError::Validation(
            "Unsupported image type. Use PNG, JPG or WebP.".to_string(),
        ));
    }
    match base64_decoded_len(image_base64) {
        None => Err(AppError::Validation("That image could not be read.".to_string())),
        Some(bytes) if bytes > MAX_IMAGE_BYTES => {
            Err(AppError::Validation("Image is too large.".to_string()))
        }
        Some(_) => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Response validation (pure - see this file's tests)
// ---------------------------------------------------------------------------

/// Anything that isn't one of the 3 documented levels becomes `low`. The
/// schema already constrains this to an enum, so in practice this only
/// fires if a future model or a schema change lets something else through -
/// and the safe direction to fail in is "flag it for review," never "show it
/// as trustworthy."
fn normalize_confidence(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" => "high".to_string(),
        "medium" => "medium".to_string(),
        _ => "low".to_string(),
    }
}

/// The last line of defence between the model and the form, and the reason
/// a malformed-but-schema-valid reply can't quietly become data:
///
///   - a field name outside this kind's list is DROPPED (the model cannot
///     invent a field the form has no slot for),
///   - a duplicate field name keeps only the first occurrence (so a later,
///     contradictory value can't silently win),
///   - an empty or whitespace-only value collapses to `None`, which is the
///     same "not in the image" state as an explicit null rather than a blank
///     that would overwrite an existing form value with nothing,
///   - confidence is normalised,
///   - a group with nothing in it at all is dropped, and seat labels are
///     trimmed and de-blanked.
///
/// Pure and network-free on purpose - the same convention `ai_categorize`
/// follows, and what lets every rule above be unit tested without an API key.
fn sanitize_result(kind: &str, raw: AiImportResult) -> AppResult<AiImportResult> {
    let allowed = field_names_for_kind(kind)?;
    let mut seen: Vec<String> = Vec::new();
    let mut fields: Vec<AiImportField> = Vec::new();
    for f in raw.fields {
        if !allowed.contains(&f.field.as_str()) || seen.contains(&f.field) {
            continue;
        }
        seen.push(f.field.clone());
        let value = f
            .value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        fields.push(AiImportField {
            field: f.field,
            value,
            confidence: normalize_confidence(&f.confidence),
        });
    }

    let ticket_groups = raw
        .ticket_groups
        .into_iter()
        .map(|g| AiImportTicketGroup {
            quantity: clean(g.quantity),
            tier: clean(g.tier),
            section: clean(g.section),
            row: clean(g.row),
            ticket_type: clean(g.ticket_type),
            unit_price: clean(g.unit_price),
            fees: clean(g.fees),
            seats: g
                .seats
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        })
        .filter(|g| {
            g.quantity.is_some()
                || g.tier.is_some()
                || g.section.is_some()
                || g.row.is_some()
                || g.ticket_type.is_some()
                || g.unit_price.is_some()
                || g.fees.is_some()
                || !g.seats.is_empty()
        })
        .collect();

    Ok(AiImportResult { readable: raw.readable, fields, ticket_groups })
}

fn clean(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// The one network call
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type", default)]
    block_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

/// Pulls the model's JSON out of a Messages response.
///
/// It has to look for the first block whose `type` is `"text"` rather than
/// taking `content[0]`: `ANTHROPIC_MODEL` runs with thinking on by default,
/// so the first block is frequently a `thinking` block, which deserialises
/// here with an empty `text` and would otherwise be handed to the JSON
/// parser as `""`. That failure mode looks exactly like "the API broke" and
/// is entirely self-inflicted, so it is checked rather than assumed.
fn first_text_block(blocks: &[AnthropicContentBlock]) -> Option<&str> {
    blocks
        .iter()
        .find(|b| b.block_type == "text" && !b.text.trim().is_empty())
        .map(|b| b.text.as_str())
}

fn send_anthropic_request(
    api_key: &str,
    body: &serde_json::Value,
) -> Result<String, (Option<reqwest::StatusCode>, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(ANTHROPIC_TIMEOUT_SECS))
        .build()
        .map_err(|e| (None, format!("could not build the HTTP client: {e}")))?;
    let resp = client
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .header("content-type", "application/json")
        .json(body)
        .send()
        .map_err(|e| (None, format!("could not reach Anthropic: {e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .map_err(|e| (Some(status), format!("could not read Anthropic's response: {e}")))?;
    if !status.is_success() {
        return Err((Some(status), text));
    }
    Ok(text)
}

/// Builds the request body. Split out from the call itself so the exact
/// wire shape is greppable and testable without a key.
///
/// Two things here are deliberate and worth not "tidying" later:
///   - there is NO `temperature` field. `ai_categorize` sends
///     `temperature: 0.0`, which is correct for the model IT uses, but
///     `ANTHROPIC_MODEL` here rejects sampling parameters outright (HTTP
///     400). Copying that line across would break every request.
///   - `output_config` carries BOTH `format` (the strict schema) and
///     `effort` - they are siblings inside one object, not two top-level
///     parameters.
fn build_request_body(kind: &str, media_type: &str, image_base64: &str) -> AppResult<serde_json::Value> {
    Ok(json!({
        "model": ANTHROPIC_MODEL,
        "max_tokens": ANTHROPIC_MAX_TOKENS,
        "output_config": {
            "effort": ANTHROPIC_EFFORT,
            "format": { "type": "json_schema", "schema": build_schema(kind)? }
        },
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": image_base64
                    }
                },
                { "type": "text", "text": build_prompt(kind)? }
            ]
        }]
    }))
}

/// The whole analysis, synchronously. Every error message here is one of
/// the short, non-technical strings marko asked for in section 13 - no
/// status codes, no response bodies, no stack traces reach the UI. The
/// detail is not lost, it is simply not his problem: a wrong API key and a
/// DNS failure both read as "AI analysis failed. Try again." because the
/// action he can take is identical either way.
fn analyze_impl(kind: &str, media_type: &str, image_base64: &str) -> AppResult<AiImportResult> {
    // Cheapest rejections first, so a bad IPC call never reaches the network
    // and never builds a schema it won't use.
    field_names_for_kind(kind)?;
    validate_image(media_type, image_base64)?;
    let body = build_request_body(kind, media_type, image_base64)?;

    let api_key = embedded_anthropic_api_key().ok_or_else(|| {
        AppError::External("AI import isn't available in this build.".to_string())
    })?;

    // Exactly one call, plus at most one retry on a transient status - see
    // this module's doc comment on cost control.
    let mut attempt = send_anthropic_request(&api_key, &body);
    if let Err((Some(status), _)) = attempt {
        if is_retriable_anthropic_status(status) {
            std::thread::sleep(std::time::Duration::from_millis(750));
            attempt = send_anthropic_request(&api_key, &body);
        }
    }
    let text = attempt.map_err(|_| AppError::External("AI analysis failed. Try again.".to_string()))?;

    let parsed: AnthropicResponse = serde_json::from_str(&text)
        .map_err(|_| AppError::External("AI analysis failed. Try again.".to_string()))?;

    // A truncated reply is JSON that stops mid-object; parsing it would
    // either fail confusingly or (worse) succeed on a partial structure.
    if parsed.stop_reason.as_deref() == Some("max_tokens") {
        return Err(AppError::External(
            "There was too much on that screenshot to read in one go. Try a smaller crop."
                .to_string(),
        ));
    }
    if parsed.stop_reason.as_deref() == Some("refusal") {
        return Err(AppError::External(
            "Could not reliably identify required fields.".to_string(),
        ));
    }

    let raw_json = first_text_block(&parsed.content)
        .ok_or_else(|| AppError::External("AI analysis failed. Try again.".to_string()))?;
    let raw: AiImportResult = serde_json::from_str(raw_json)
        .map_err(|_| AppError::External("Could not reliably identify required fields.".to_string()))?;

    sanitize_result(kind, raw)
}

/// The single command this module exposes. Read-only, database-free, and
/// only ever called from an explicit click/drop/paste in the AI Import
/// panel - never on mount, never on a timer, never in the background.
///
/// `async fn` + `spawn_blocking` for the same reason every sheet-sync
/// command became one in 2.3.5: this makes a blocking network call, and a
/// plain `#[tauri::command] fn` would run it on Tauri's single main/IPC
/// thread and freeze the entire UI for as long as it takes. Unlike those
/// commands this one needs no `AppState` at all (it touches no database),
/// so the closure captures only owned `String`s.
#[tauri::command]
pub async fn analyze_import_image(
    kind: String,
    media_type: String,
    image_base64: String,
) -> AppResult<AiImportResult> {
    tauri::async_runtime::spawn_blocking(move || analyze_impl(&kind, &media_type, &image_base64))
        .await
        .map_err(|_| AppError::External("AI analysis failed. Try again.".to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, value: Option<&str>, confidence: &str) -> AiImportField {
        AiImportField {
            field: name.to_string(),
            value: value.map(|v| v.to_string()),
            confidence: confidence.to_string(),
        }
    }

    fn empty_group() -> AiImportTicketGroup {
        AiImportTicketGroup {
            quantity: None,
            tier: None,
            section: None,
            row: None,
            ticket_type: None,
            unit_price: None,
            fees: None,
            seats: Vec::new(),
        }
    }

    // -- kinds ------------------------------------------------------------

    #[test]
    fn the_three_supported_kinds_each_have_their_own_field_list() {
        assert!(field_names_for_kind("event").unwrap().contains(&"venue"));
        assert!(field_names_for_kind("order").unwrap().contains(&"orderReference"));
        assert!(field_names_for_kind("sale").unwrap().contains(&"salePrice"));
    }

    #[test]
    fn an_unknown_kind_is_rejected_before_anything_else_happens() {
        assert!(field_names_for_kind("ticket").is_err());
        assert!(build_schema("").is_err());
        assert!(build_request_body("listing", "image/png", "AAAA").is_err());
    }

    #[test]
    fn a_sale_never_asks_for_seat_or_ticket_fields() {
        // A sale is recorded against an existing ticket row (SaleInput.
        // ticketId), so extracted seat text would have nowhere valid to go -
        // see field_names_for_kind's own comment.
        let sale = field_names_for_kind("sale").unwrap();
        assert!(!sale.contains(&"section"));
        assert!(!sale.contains(&"row"));
        assert!(!sale.contains(&"seats"));
        assert!(!sale.contains(&"tier"));
    }

    // -- image validation (marko's section 11) ----------------------------

    #[test]
    fn decoded_length_is_computed_without_decoding() {
        assert_eq!(base64_decoded_len("AAAA"), Some(3));
        assert_eq!(base64_decoded_len("AAA="), Some(2));
        assert_eq!(base64_decoded_len("AA=="), Some(1));
        assert_eq!(base64_decoded_len("AAAAAAAA"), Some(6));
    }

    #[test]
    fn a_length_that_cannot_be_base64_is_rejected_rather_than_guessed_at() {
        assert_eq!(base64_decoded_len(""), None);
        assert_eq!(base64_decoded_len("AAA"), None);
        assert_eq!(base64_decoded_len("AAAAA"), None);
    }

    #[test]
    fn png_jpeg_and_webp_are_accepted_and_everything_else_is_not() {
        assert!(validate_image("image/png", "AAAA").is_ok());
        assert!(validate_image("image/jpeg", "AAAA").is_ok());
        assert!(validate_image("image/webp", "AAAA").is_ok());
        // marko asked for exactly these three - a PDF or a GIF is a clear,
        // early "no", not something to send and let Anthropic decide.
        assert!(validate_image("application/pdf", "AAAA").is_err());
        assert!(validate_image("image/gif", "AAAA").is_err());
        assert!(validate_image("text/plain", "AAAA").is_err());
    }

    #[test]
    fn an_image_over_the_size_ceiling_is_refused_without_a_request() {
        // 4 base64 chars per 3 bytes, so this is comfortably over 5 MB.
        let too_big = "A".repeat(4 * (MAX_IMAGE_BYTES / 3 + 1024));
        let err = validate_image("image/png", &too_big).unwrap_err();
        assert!(format!("{err}").contains("too large"), "got: {err}");
    }

    #[test]
    fn an_empty_payload_is_refused() {
        assert!(validate_image("image/png", "").is_err());
    }

    // -- confidence -------------------------------------------------------

    #[test]
    fn the_three_documented_confidence_levels_survive_unchanged() {
        assert_eq!(normalize_confidence("high"), "high");
        assert_eq!(normalize_confidence("medium"), "medium");
        assert_eq!(normalize_confidence("low"), "low");
        assert_eq!(normalize_confidence("  HIGH  "), "high");
    }

    #[test]
    fn an_unrecognized_confidence_degrades_to_low_never_to_high() {
        // The safe direction to fail in: flag it for review.
        assert_eq!(normalize_confidence("very high"), "low");
        assert_eq!(normalize_confidence("certain"), "low");
        assert_eq!(normalize_confidence(""), "low");
    }

    // -- sanitize_result --------------------------------------------------

    #[test]
    fn a_field_the_form_has_no_slot_for_is_dropped() {
        let raw = AiImportResult {
            readable: true,
            fields: vec![
                field("venue", Some("Olympiahalle"), "high"),
                // Not in the event list - the model must not be able to
                // invent a field.
                field("pricePerTicket", Some("180"), "high"),
            ],
            ticket_groups: vec![],
        };
        let out = sanitize_result("event", raw).unwrap();
        assert_eq!(out.fields.len(), 1);
        assert_eq!(out.fields[0].field, "venue");
    }

    #[test]
    fn a_duplicated_field_keeps_only_the_first_value() {
        let raw = AiImportResult {
            readable: true,
            fields: vec![
                field("name", Some("Coldplay Munich"), "high"),
                field("name", Some("Something else"), "low"),
            ],
            ticket_groups: vec![],
        };
        let out = sanitize_result("event", raw).unwrap();
        assert_eq!(out.fields.len(), 1);
        assert_eq!(out.fields[0].value.as_deref(), Some("Coldplay Munich"));
    }

    #[test]
    fn a_missing_value_stays_null_and_is_never_filled_in() {
        // marko's own rule - a field that isn't in the image comes back
        // null, and nothing downstream may substitute a default.
        let raw = AiImportResult {
            readable: true,
            fields: vec![field("city", None, "low"), field("venue", Some("   "), "high")],
            ticket_groups: vec![],
        };
        let out = sanitize_result("event", raw).unwrap();
        assert_eq!(out.fields.len(), 2);
        assert_eq!(out.fields[0].value, None);
        // A whitespace-only value is the same thing as absent - it must not
        // become a blank that overwrites what's already in the form.
        assert_eq!(out.fields[1].value, None);
    }

    #[test]
    fn values_are_trimmed_but_never_reformatted() {
        let raw = AiImportResult {
            readable: true,
            fields: vec![field("eventDate", Some("  14 Sep 2026 "), "high")],
            ticket_groups: vec![],
        };
        let out = sanitize_result("event", raw).unwrap();
        // Trimmed, but NOT parsed into an ISO date here - the form and the
        // existing backend validation own that.
        assert_eq!(out.fields[0].value.as_deref(), Some("14 Sep 2026"));
    }

    #[test]
    fn two_ticket_groups_are_kept_separate_and_never_merged() {
        // marko's own example, and his explicit "Nechcem zlievať rôzne
        // groups do jednej".
        let raw = AiImportResult {
            readable: true,
            fields: vec![],
            ticket_groups: vec![
                AiImportTicketGroup {
                    quantity: Some("4".into()),
                    tier: Some("100".into()),
                    section: Some("102".into()),
                    row: Some("14".into()),
                    unit_price: Some("180".into()),
                    seats: vec!["21".into(), "22".into(), "23".into(), "24".into()],
                    ..empty_group()
                },
                AiImportTicketGroup {
                    quantity: Some("2".into()),
                    tier: Some("200".into()),
                    section: Some("205".into()),
                    row: Some("8".into()),
                    unit_price: Some("120".into()),
                    seats: vec!["11".into(), "12".into()],
                    ..empty_group()
                },
            ],
        };
        let out = sanitize_result("order", raw).unwrap();
        assert_eq!(out.ticket_groups.len(), 2);
        assert_eq!(out.ticket_groups[0].section.as_deref(), Some("102"));
        assert_eq!(out.ticket_groups[0].seats.len(), 4);
        assert_eq!(out.ticket_groups[1].section.as_deref(), Some("205"));
        assert_eq!(out.ticket_groups[1].seats, vec!["11", "12"]);
    }

    #[test]
    fn blank_seat_labels_are_dropped_and_the_rest_are_trimmed() {
        let raw = AiImportResult {
            readable: true,
            fields: vec![],
            ticket_groups: vec![AiImportTicketGroup {
                quantity: Some("2".into()),
                seats: vec![" 21 ".into(), "".into(), "  ".into(), "22".into()],
                ..empty_group()
            }],
        };
        let out = sanitize_result("order", raw).unwrap();
        assert_eq!(out.ticket_groups[0].seats, vec!["21", "22"]);
    }

    #[test]
    fn a_completely_empty_ticket_group_is_dropped() {
        let raw = AiImportResult {
            readable: true,
            fields: vec![],
            ticket_groups: vec![empty_group(), AiImportTicketGroup { section: Some("A".into()), ..empty_group() }],
        };
        let out = sanitize_result("order", raw).unwrap();
        assert_eq!(out.ticket_groups.len(), 1);
        assert_eq!(out.ticket_groups[0].section.as_deref(), Some("A"));
    }

    #[test]
    fn an_unreadable_verdict_is_passed_through_untouched() {
        let raw = AiImportResult { readable: false, fields: vec![], ticket_groups: vec![] };
        let out = sanitize_result("order", raw).unwrap();
        assert!(!out.readable);
    }

    // -- response parsing -------------------------------------------------

    #[test]
    fn the_json_is_read_from_the_first_text_block_not_the_first_block() {
        // The configured model runs with thinking on, so a thinking block
        // routinely arrives first and deserializes here with empty text.
        let blocks = vec![
            AnthropicContentBlock { block_type: "thinking".into(), text: String::new() },
            AnthropicContentBlock { block_type: "text".into(), text: "{\"readable\":true}".into() },
        ];
        assert_eq!(first_text_block(&blocks), Some("{\"readable\":true}"));
    }

    #[test]
    fn a_response_with_no_usable_text_block_yields_none() {
        assert_eq!(first_text_block(&[]), None);
        let only_thinking =
            vec![AnthropicContentBlock { block_type: "thinking".into(), text: "   ".into() }];
        assert_eq!(first_text_block(&only_thinking), None);
    }

    #[test]
    fn a_malformed_ai_response_does_not_deserialize_into_a_result() {
        // What a truncated or non-JSON reply actually looks like at this
        // layer - analyze_impl turns each of these into marko's short
        // "Could not reliably identify required fields." message.
        assert!(serde_json::from_str::<AiImportResult>("not json at all").is_err());
        assert!(serde_json::from_str::<AiImportResult>("{\"readable\":true,\"fields\":[").is_err());
        // Missing required keys is also a failure, not a partial success.
        assert!(serde_json::from_str::<AiImportResult>("{\"readable\":true}").is_err());
    }

    #[test]
    fn a_well_formed_ai_response_round_trips_through_the_camel_case_wire_shape() {
        let json = r#"{
            "readable": true,
            "fields": [{"field":"section","value":"402","confidence":"high"}],
            "ticketGroups": [{
                "quantity":"2","tier":"100","section":"102","row":"14",
                "ticketType":"E-ticket","unitPrice":"180","fees":null,
                "seats":["21","22"]
            }]
        }"#;
        let parsed: AiImportResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.fields[0].field, "section");
        assert_eq!(parsed.ticket_groups[0].ticket_type.as_deref(), Some("E-ticket"));
        assert_eq!(parsed.ticket_groups[0].fees, None);
    }

    // -- request shape ----------------------------------------------------

    #[test]
    fn the_request_body_has_the_exact_shape_this_model_requires() {
        let body = build_request_body("order", "image/png", "AAAA").unwrap();
        assert_eq!(body["model"], ANTHROPIC_MODEL);
        // Sampling parameters are rejected by this model - sending one
        // would 400 every single request.
        assert!(body.get("temperature").is_none(), "temperature must never be sent");
        // effort and format are siblings inside output_config, not
        // top-level parameters.
        assert_eq!(body["output_config"]["effort"], ANTHROPIC_EFFORT);
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert!(body.get("output_format").is_none(), "output_format is the deprecated spelling");
        // Image first, then the instruction.
        assert_eq!(body["messages"][0]["content"][0]["type"], "image");
        assert_eq!(body["messages"][0]["content"][0]["source"]["media_type"], "image/png");
        assert_eq!(body["messages"][0]["content"][1]["type"], "text");
    }

    #[test]
    fn every_schema_object_is_closed_and_fully_required() {
        // Both are hard requirements of structured-output mode - a missing
        // additionalProperties:false or an incomplete `required` is a 400,
        // and it is easy to break by adding one property later.
        let schema = build_schema("order").unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"], json!(["readable", "fields", "ticketGroups"]));
        let group = &schema["properties"]["ticketGroups"]["items"];
        assert_eq!(group["additionalProperties"], false);
        let props = group["properties"].as_object().unwrap();
        let required = group["required"].as_array().unwrap();
        assert_eq!(props.len(), required.len(), "every group property must be required");
        let item = &schema["properties"]["fields"]["items"];
        assert_eq!(item["additionalProperties"], false);
        assert_eq!(item["properties"]["confidence"]["enum"], json!(["high", "medium", "low"]));
    }

    #[test]
    fn the_schema_only_offers_the_field_names_this_kind_actually_has() {
        // This is what stops a hallucinated field name at the API boundary,
        // before sanitize_result even has to drop it.
        let schema = build_schema("event").unwrap();
        let names = schema["properties"]["fields"]["items"]["properties"]["field"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(names.contains(&"venue".to_string()));
        assert!(!names.contains(&"orderReference".to_string()));
    }

    #[test]
    fn the_prompt_states_the_no_guessing_rules_and_lists_every_field() {
        let prompt = build_prompt("order").unwrap();
        assert!(prompt.contains("Never guess"));
        assert!(prompt.contains("null"));
        assert!(prompt.contains("never merge"));
        for name in field_names_for_kind("order").unwrap() {
            assert!(prompt.contains(name), "prompt must name the '{name}' field");
        }
    }

    #[test]
    fn the_prompt_asks_for_iso_dates_and_forbids_assuming_a_year() {
        // The app's date inputs are <input type="date">, which silently
        // shows nothing for "14 Sep 2026" - so normalising the FORMAT is
        // required, while inventing a missing YEAR is still forbidden.
        for kind in ["event", "order", "sale"] {
            let prompt = build_prompt(kind).unwrap();
            assert!(prompt.contains("YYYY-MM-DD"), "{kind} prompt must ask for ISO dates");
            assert!(prompt.contains("do not assume the current"), "{kind} prompt must forbid guessing a year");
        }
    }

    #[test]
    fn the_event_prompt_forbids_deciding_a_status() {
        // marko: "AI nesmie automaticky meniť event status podľa vlastnej
        // úvahy."
        let prompt = build_prompt("event").unwrap();
        assert!(prompt.contains("literally printed"));
    }
}
