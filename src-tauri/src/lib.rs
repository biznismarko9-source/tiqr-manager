mod codes;
mod commands;
mod db;
mod error;
mod finance;
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
        .setup(|app| {
            let handle = app.handle().clone();
            let db_path = db::resolve_db_path(&handle).map_err(|e| e.to_string())?;
            let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
            db::run_migrations(&conn).map_err(|e| e.to_string())?;
            commands::demo::seed_if_empty(&conn).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db: Mutex::new(conn),
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
            commands::orders::create_order,
            commands::orders::update_order,
            commands::orders::delete_order,
            commands::tickets::list_tickets,
            commands::tickets::get_ticket,
            commands::tickets::update_ticket,
            commands::sales::list_sales,
            commands::sales::get_sale,
            commands::sales::create_sale,
            commands::sales::update_sale,
            commands::sales::delete_sale,
            commands::lookups::list_platforms,
            commands::lookups::create_platform,
            commands::lookups::delete_platform,
            commands::lookups::list_suppliers,
            commands::lookups::create_supplier,
            commands::lookups::delete_supplier,
            commands::dashboard::get_dashboard,
            commands::csv_import::preview_orders_csv,
            commands::csv_import::import_orders_csv,
            commands::csv_export::export_events_csv,
            commands::csv_export::export_orders_csv,
            commands::csv_export::export_tickets_csv,
            commands::csv_export::export_sales_csv,
            commands::csv_export::export_inventory_csv,
            commands::backup::backup_database,
            commands::backup::restore_database,
            commands::demo::clear_demo_data,
            commands::demo::reset_demo_data,
            commands::app_info::get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
