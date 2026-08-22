mod codes;
mod commands;
mod db;
mod error;
mod finance;
mod google_oauth;
mod google_sheets;
mod models;
mod money;

use db::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 2.0.5: opens the system browser for "Sign in with Google" - see
        // google_oauth.rs's module doc comment.
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let db_path = db::resolve_db_path(&handle).map_err(|e| e.to_string())?;
            let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
            db::run_migrations(&conn).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db: Mutex::new(conn),
                oauth_cancel_flag: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::list_events,
            commands::events::get_event,
            commands::events::create_event,
            commands::events::update_event,
            commands::events::delete_event,
            commands::orders::list_orders,
            commands::orders::get_order,
            commands::orders::get_order_sales_summary,
            commands::orders::create_order,
            commands::orders::update_order,
            commands::orders::delete_order,
            commands::tickets::list_tickets,
            commands::tickets::get_ticket,
            commands::tickets::update_ticket,
            commands::tickets::bulk_update_tickets,
            commands::tickets::bulk_update_ticket_status,
            commands::pulls::list_pulls,
            commands::pulls::get_pull,
            commands::pulls::create_pull,
            commands::pulls::update_pull,
            commands::pulls::delete_pull,
            commands::pulls::set_pull_transfer_done,
            commands::pulls_received::list_pulls_received,
            commands::pulls_received::get_pull_received,
            commands::pulls_received::create_pull_received,
            commands::pulls_received::update_pull_received,
            commands::pulls_received::delete_pull_received,
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
            commands::lookups::list_platforms,
            commands::lookups::create_platform,
            commands::lookups::delete_platform,
            commands::lookups::update_platform_kind,
            commands::lookups::list_suppliers,
            commands::lookups::create_supplier,
            commands::lookups::delete_supplier,
            commands::dashboard::get_dashboard,
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
            commands::settings::get_app_setting,
            commands::settings::set_app_setting,
            commands::sheets_sync::get_sheets_connection_status,
            commands::sheets_sync::set_sheets_connection,
            commands::sheets_sync::clear_sheets_connection,
            commands::sheets_sync::test_sheets_connection,
            commands::sheets_sync::detect_spreadsheet_tabs,
            commands::pulls_sheet_sync::sync_pulls,
            commands::pulls_sheet_sync::create_pulls_sheet,
            commands::orders_sheet_sync::sync_orders,
            commands::orders_sheet_sync::sync_sales,
            commands::orders_sheet_sync::create_orders_sheet,
            commands::google_auth::get_google_sign_in_status,
            commands::google_auth::start_google_sign_in,
            commands::google_auth::cancel_google_sign_in,
            commands::google_auth::google_sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
