# Changelog

Short, append-only entries, newest at the top - one entry per completed
task under the TIQR development protocol (see step 8). This is not a
replacement for the detailed `REDESIGN-X.Y.Z-REPORT.md` / `*-REPORT.md`
files at the repo root (one per release, written for marko, in Slovak) -
those still get written for real releases. This log exists so a future
session can see recent activity at a glance without opening any of them.

## 2.2.0 - Price Checker Market Analysis

New `commands/price_checker_analysis.rs` (2 commands) derives tier/section
price breakdowns, comparable-ticket ranking, and Your Tickets price
recommendations from a Visible Scanner session's already-accumulated
listings - never touches the scanner's own session/lifecycle code.
`migrations/019_price_checker_market_analysis.sql` adds `price_check_tiers`
so saved checks remember a per-tier breakdown going forward. 40 new Rust
unit/integration tests (incl. 2 added during this release's own adversarial
review pass, after finding tier/section grouping was case-sensitive while
comparable-matching already wasn't - see `PROJECT_STATE/PROTECTED_AREAS.md`'s
"2.2.0" entry). Full detail, including every flagged design decision, in
`PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md`.

## 2.2.0 - StubHub fully removed, including history

`migrations/020_remove_stubhub.sql` deletes the StubHub marketplace row and
every `price_checks`/`price_check_tiers`/`event_marketplace_links` row that
ever referenced it - marko's own explicit, confirmed decision to go further
than 2.1.6's "keep history, stop offering it for new checks." Irreversible
by design.

## 2.1.9 - PROJECT_STATE protocol adopted

Set up `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/KNOWN_BUGS.md`,
and `PROJECT_STATE/PROTECTED_AREAS.md` (moved verbatim from the old root
`PROTECTED-AREAS-NOTES.md`, which is now a pointer stub) per marko's
development protocol. No code changes. `KNOWN_BUGS.md` starts empty by
design - see its own header for why.

## 2.1.9 - Price Checker Visible Scanner

Replaced the hidden auto-check WebView with a visible one the user scans
himself. Full detail in `PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md` and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.1.9" entry.
