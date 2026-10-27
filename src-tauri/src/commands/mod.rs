pub mod ai_import;
pub mod app_info;
pub mod attention_center;
pub mod backup;
pub mod alerts;
pub mod cloud_merge;
pub mod notepad;
pub mod cloud_sync;
pub mod calendar;
pub mod csv_export;
pub mod csv_import;
pub mod currency;
pub mod dashboard;
pub mod data_health;
pub mod database;
// 2.113.0: every Discord message TIQR posts, built and sent in one place.
pub mod discord;
pub mod event_categories;
pub mod events;
pub mod finance_accounts;
pub mod finance_entries;
pub mod finance_forecast;
pub mod finance_recurring;
pub mod firebase_google_auth;
pub mod google_auth;
pub mod inventory_intelligence;
pub mod live_sync;
pub mod lookups;
// 2.100.0: Mail Intelligence (phase 1) - read a mailbox, classify what
// arrives. Writes no business record; see its module doc comment.
pub mod mail;
pub mod mail_share;
pub mod mail_sync;
// 2.113.0: the mail reader - safe HTML, pictures and attachments in TIQR, translate.
pub mod mail_view;
// 2.113.1: the few lines of a mail that matter - where tickets go, how much.
pub mod mail_facts;
/// 2.114.0: sorting Promotions into junk and keep - free rules first, the
/// cheapest model for the rest, on a click, nothing moved without marko.
pub mod mail_cleanup;
pub mod shared_settings;
pub mod notes;
pub mod notifications;
pub mod orders;
pub mod orders_sheet_sync;
pub mod price_checker;
pub mod price_checker_analysis;
pub mod price_checker_scanner;
pub mod pulls;
pub mod pulls_received;
pub mod pulls_sheet_sync;
pub mod sales;
pub mod settings;
pub mod share;
pub mod sheets_sync;
pub mod ticket_control_center;
pub mod ticket_listings;
pub mod tickets;
pub mod workspace;
