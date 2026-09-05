mod ai_categorize;
mod codes;
mod commands;
mod db;
mod error;
mod finance;
mod fx;
mod google_oauth;
mod google_sheets;
mod models;
mod money;

use db::AppState;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 2.0.52: MUST be the first plugin registered (the plugin's own
        // documented requirement) - see the Cargo.toml comment on this same
        // dependency for why single-instance matters for this app
        // specifically (concurrent SQLite writers, not just a UI nuisance).
        // This callback runs in the ALREADY-RUNNING instance whenever a
        // second launch is attempted; that second process exits right after
        // calling it, before it ever reaches the .setup() below - so
        // db::open_connection only ever runs in the one surviving process.
        // argv/cwd from the second attempt are ignored (`_argv`/`_cwd`) -
        // this app has no CLI arguments or file-association behavior that
        // would need them, it only needs to bring the real window forward.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 2.0.5: opens the system browser for "Sign in with Google" - see
        // google_oauth.rs's module doc comment.
        .plugin(tauri_plugin_opener::init())
        // 2.5.2: registers this app's `tiqrmanager://` URL scheme for the new
        // "Forgot password?" flow - see lib/firebase.ts's
        // PASSWORD_RESET_ACTION_CODE_SETTINGS (frontend) for why a deep link
        // exists at all, and Cargo.toml's own comment on this same
        // dependency for why tauri-plugin-single-instance above needed its
        // `deep-link` feature turned on for this to reach an already-running
        // instance on Windows. The actual URL is parsed entirely on the
        // frontend (App.tsx, via @tauri-apps/plugin-deep-link's
        // getCurrent/onOpenUrl) - nothing Rust-side needs to read it.
        .plugin(tauri_plugin_deep_link::init())
        // 2.0.76: desktop notifications for the new outbound-notification
        // feature (commands/notifications.rs) - no ordering constraint
        // against the other plugins here (unlike single-instance above).
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 2.5.2: a `cargo tauri dev` build has no installer to have
            // registered `tiqrmanager://` with the OS - tauri-plugin-deep-
            // link's own documented Windows/Linux limitation is that deep
            // links only trigger for installed applications otherwise.
            // register_all() registers this build's configured schemes
            // (tauri.conf.json's `plugins.deep-link.desktop.schemes`)
            // against THIS running executable instead, purely so the
            // "Forgot password?" flow can be clicked through end to end on
            // a dev build too. Release builds get the real registration for
            // free from the NSIS installer and don't need this - best-
            // effort (`let _ =`) since a dev build failing to self-register
            // should never stop the app from starting.
            #[cfg(all(desktop, debug_assertions))]
            {
                let _ = app.deep_link().register_all();
            }
            let handle = app.handle().clone();
            let db_path = db::resolve_db_path(&handle).map_err(|e| e.to_string())?;
            let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
            db::run_migrations(&conn).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db: Mutex::new(conn),
                // 2.0.72: this is the ONE global/legacy file, opened eagerly
                // here exactly as before - it's what serves the one command
                // that already fires before anyone is signed in (theme
                // preference on the Welcome screen). The real per-account
                // file (this legacy one, or a brand-new per-uid one) is
                // switched in immediately after sign-in + approval by
                // commands::database::switch_active_database - see that
                // module's own doc comment for the full design.
                db_path: Mutex::new(db_path),
                oauth_cancel_flag: Mutex::new(None),
                firebase_oauth_cancel_flag: Mutex::new(None),
                price_scanner_sessions: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::list_events,
            commands::events::get_event,
            commands::events::create_event,
            commands::events::update_event,
            commands::events::delete_event,
            commands::events::bulk_delete_events,
            commands::events::detect_event_categories,
            commands::orders::list_orders,
            commands::orders::get_order,
            commands::orders::get_order_sales_summary,
            commands::orders::create_order,
            commands::orders::update_order,
            commands::orders::delete_order,
            commands::orders::bulk_delete_orders,
            commands::orders::bulk_set_orders_delivery_status,
            commands::orders::bulk_set_orders_payment_status,
            commands::orders::convert_order_currency,
            commands::orders::convert_currencies_to_eur,
            commands::tickets::list_tickets,
            commands::tickets::get_ticket,
            commands::tickets::update_ticket,
            commands::tickets::bulk_update_tickets,
            commands::tickets::bulk_update_ticket_status,
            commands::tickets::bulk_update_ticket_delivery_status,
            commands::tickets::bulk_update_ticket_resale_status,
            commands::tickets::list_ticket_types,
            commands::ticket_control_center::list_control_center_tickets,
            commands::ticket_listings::list_ticket_listings_for_event,
            commands::ticket_listings::create_ticket_listing,
            commands::ticket_listings::update_ticket_listing,
            commands::ticket_listings::delete_ticket_listing,
            commands::ticket_listings::bulk_update_ticket_listings_status,
            commands::ticket_listings::bulk_update_ticket_listings_price,
            commands::ticket_listings::bulk_delete_ticket_listings,
            commands::inventory_intelligence::get_inventory_intelligence,
            commands::attention_center::get_attention_center,
            commands::pulls::list_pulls,
            commands::pulls::get_pull,
            commands::pulls::create_pull,
            commands::pulls::update_pull,
            commands::pulls::delete_pull,
            commands::pulls::bulk_delete_pulls,
            commands::pulls::set_pull_transfer_done,
            commands::pulls_received::list_pulls_received,
            commands::pulls_received::get_pull_received,
            commands::pulls_received::create_pull_received,
            commands::pulls_received::update_pull_received,
            commands::pulls_received::delete_pull_received,
            commands::pulls_received::bulk_delete_pulls_received,
            commands::pulls_received::link_pull_received_to_order,
            commands::pulls_received::list_pulls_received_for_order,
            commands::sales::list_sales,
            commands::sales::list_sale_groups,
            commands::sales::list_sale_currencies,
            commands::sales::list_sales_by_group,
            commands::sales::get_sale,
            commands::sales::create_sale,
            commands::sales::create_sales_batch,
            commands::sales::update_sale,
            commands::sales::bulk_update_sale_payment_status,
            commands::sales::refund_sale,
            commands::sales::delete_sale,
            commands::sales::delete_sale_group,
            commands::sales::bulk_delete_sale_groups,
            commands::sales::bulk_set_sale_groups_delivery_status,
            commands::sales::bulk_set_sale_groups_payment_status,
            commands::lookups::list_platforms,
            commands::lookups::create_platform,
            commands::lookups::delete_platform,
            commands::lookups::update_platform_kind,
            commands::lookups::list_suppliers,
            commands::lookups::create_supplier,
            commands::lookups::delete_supplier,
            commands::event_categories::list_event_categories,
            commands::event_categories::create_event_category,
            commands::event_categories::delete_event_category,
            commands::finance_entries::list_finance_categories,
            commands::finance_entries::create_finance_category,
            commands::finance_entries::delete_finance_category,
            commands::finance_entries::list_finance_entries,
            commands::finance_entries::list_finance_entries_for_order,
            commands::finance_entries::create_finance_entry,
            commands::finance_entries::update_finance_entry,
            commands::finance_entries::delete_finance_entry,
            commands::finance_accounts::list_accounts,
            commands::finance_accounts::create_account,
            commands::finance_accounts::update_account,
            commands::finance_accounts::delete_account,
            commands::finance_accounts::set_account_balance,
            commands::finance_accounts::list_transfers,
            commands::finance_accounts::create_transfer,
            commands::finance_accounts::delete_transfer,
            commands::finance_recurring::list_recurring_expenses,
            commands::finance_recurring::create_recurring_expense,
            commands::finance_recurring::update_recurring_expense,
            commands::finance_recurring::delete_recurring_expense,
            commands::finance_recurring::pause_recurring_expense,
            commands::finance_recurring::resume_recurring_expense,
            commands::finance_recurring::skip_recurring_expense,
            commands::finance_recurring::create_from_recurring,
            commands::finance_forecast::get_cashflow_forecast,
            commands::dashboard::get_dashboard,
            commands::calendar::get_calendar,
            commands::csv_import::preview_orders_csv,
            commands::csv_import::import_orders_csv,
            commands::csv_export::export_events_csv,
            commands::csv_export::export_events_csv_selected,
            commands::csv_export::export_orders_csv,
            commands::csv_export::export_orders_csv_selected,
            commands::csv_export::export_tickets_csv,
            commands::csv_export::export_tickets_csv_selected,
            commands::csv_export::export_sales_csv,
            commands::csv_export::export_sales_csv_selected,
            commands::csv_export::export_inventory_csv,
            commands::csv_export::export_orders_csv_template,
            commands::backup::backup_database,
            commands::backup::validate_backup_file,
            commands::backup::restore_database,
            commands::app_info::get_app_info,
            commands::database::switch_active_database,
            commands::notifications::get_notification_status,
            commands::notifications::set_notification_config,
            commands::notifications::test_desktop_notification,
            commands::notifications::test_ntfy_notification,
            commands::notifications::check_and_send_notifications,
            commands::settings::get_app_setting,
            commands::settings::set_app_setting,
            commands::settings::get_anthropic_api_key_configured,
            commands::settings::set_anthropic_api_key,
            commands::sheets_sync::get_sheets_connection_status,
            commands::sheets_sync::set_sheets_connection,
            commands::sheets_sync::clear_sheets_connection,
            commands::sheets_sync::test_sheets_connection,
            commands::sheets_sync::detect_spreadsheet_tabs,
            commands::pulls_sheet_sync::sync_pulls,
            commands::pulls_sheet_sync::push_pulls,
            commands::pulls_sheet_sync::create_pulls_sheet,
            commands::pulls_sheet_sync::setup_pulls_sheet,
            commands::orders_sheet_sync::sync_orders,
            commands::orders_sheet_sync::push_orders,
            commands::orders_sheet_sync::sync_sales,
            commands::orders_sheet_sync::push_sales,
            commands::orders_sheet_sync::force_push_sales,
            commands::orders_sheet_sync::create_orders_sheet,
            commands::orders_sheet_sync::setup_orders_sheet,
            commands::google_auth::get_google_sign_in_status,
            commands::google_auth::start_google_sign_in,
            commands::google_auth::cancel_google_sign_in,
            commands::google_auth::google_sign_out,
            commands::firebase_google_auth::firebase_google_sign_in_available,
            commands::firebase_google_auth::start_firebase_google_sign_in,
            commands::firebase_google_auth::cancel_firebase_google_sign_in,
            commands::currency::convert_currency,
            commands::price_checker::list_marketplaces,
            commands::price_checker::create_marketplace,
            commands::price_checker::delete_marketplace,
            commands::price_checker::save_event_marketplace_link,
            commands::price_checker::save_price_check,
            commands::price_checker::get_price_checker_summary,
            commands::price_checker_scanner::open_price_scanner,
            commands::price_checker_scanner::scan_visible_prices,
            commands::price_checker_scanner::cancel_price_scan,
            commands::price_checker_scanner::close_price_scanner,
            commands::price_checker_analysis::compute_market_analysis,
            commands::price_checker_analysis::compute_comparable_market,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 2.1.3 (originally the hidden auto-check's own hardening,
            // adapted 2.1.9 for the Visible Scanner's session map).
            // `ExitRequested` fires once, right as the app is about to exit
            // (confirmed against Tauri's own published docs, not memory -
            // docs.rs/tauri's `RunEvent` and `App::run`). BEST-EFFORT,
            // NON-BLOCKING only: never calls `api.prevent_exit()`, so it can
            // never delay or interrupt an ordinary close - the OS reclaims
            // every window/handle this process owns on exit regardless of
            // whether this fires. Its only real effect is flipping every
            // still-open scanner session's cancel flag a little sooner than
            // process teardown alone would, in case a scan eval happens to
            // be in flight at that exact moment - it does NOT close the
            // scanner windows themselves, that's unnecessary (see this
            // block's own reasoning above: the OS reclaims them anyway) and
            // Visible Scanner windows are ordinary, user-visible windows the
            // OS already knows how to tear down normally, unlike the old
            // hidden reader this replaced. Deliberately scoped to ONLY this
            // feature's own sessions - not `oauth_cancel_flag`/
            // `firebase_oauth_cancel_flag`, a separate, unrelated feature.
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(sessions) = state.price_scanner_sessions.lock() {
                        for session in sessions.values() {
                            session.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
        });
}
