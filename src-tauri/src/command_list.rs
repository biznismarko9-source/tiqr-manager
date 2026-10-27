//! 2.118.0: every command the screens call, in one list that both builds
//! read. The computer's `run()` (lib.rs) hands it to Tauri's
//! `generate_handler!`; the phone build (phone.rs) hands it to its own
//! (shim/macros), which calls the same functions by the same names.
//!
//! A command marked `#[cfg(not(target_arch = "wasm32"))]` is the computer's
//! alone: it needs a window, a file on the computer, a thread of its own, or
//! the Google account connected on the computer (Drive, Sheets, Gmail's
//! sign-in). On the phone, asking for one answers that it works in TIQR on
//! the computer (phone.rs).
//!
//! A new command goes here, once; nothing else lists commands.

macro_rules! tiqr_commands {
    ($($handler:tt)+) => {
        $($handler)+![
            commands::events::list_events,
            commands::events::get_event,
            commands::events::create_event,
            commands::events::update_event,
            commands::events::delete_event,
            commands::events::bulk_delete_events,
            commands::events::detect_event_categories,
            // 2.116.0: the log, crash notes and the diagnostic report.
            diag::report_client_error,
            diag::take_crash_report,
            diag::export_diagnostics,
            #[cfg(not(target_arch = "wasm32"))]
            diag::save_diagnostics,
            // 2.117.0: Web access - the Settings card on the computer that serves.
            #[cfg(not(target_arch = "wasm32"))]
            web_access::get_web_access_status,
            #[cfg(not(target_arch = "wasm32"))]
            web_access::set_web_access_options,
            // 2.7.0: AI Import Assistant. Read-only and database-free by
            // construction - see commands/ai_import.rs's module doc comment.
            commands::ai_import::analyze_import_image,
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
            commands::pulls::set_pull_paid,
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
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_import::preview_orders_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_import::import_orders_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_events_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_events_csv_selected,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_orders_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_orders_csv_selected,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_tickets_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_tickets_csv_selected,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_sales_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_sales_csv_selected,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_inventory_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::csv_export::export_orders_csv_template,
            #[cfg(not(target_arch = "wasm32"))]
            commands::backup::backup_database,
            #[cfg(not(target_arch = "wasm32"))]
            commands::backup::validate_backup_file,
            #[cfg(not(target_arch = "wasm32"))]
            commands::backup::restore_database,
            #[cfg(not(target_arch = "wasm32"))]
            commands::backup::list_restore_points,
            #[cfg(not(target_arch = "wasm32"))]
            commands::share::save_png_file,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_status,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::set_cloud_sync_enabled,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_push,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_pull,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_auto,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_merge::cloud_merge_pull,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_merge::cloud_merge_history,
            // 2.112.0: live sync between computers (commands/live_sync.rs).
            commands::live_sync::live_sync_status,
            commands::live_sync::live_sync_set_enabled,
            commands::live_sync::live_sync_take,
            commands::live_sync::live_sync_ack,
            commands::live_sync::live_sync_apply,
            commands::live_sync::live_sync_apply_settings,
            commands::live_sync::live_sync_set_cursor,
            commands::live_sync::live_sync_report,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_revisions,
            #[cfg(not(target_arch = "wasm32"))]
            commands::cloud_sync::cloud_sync_restore_revision,
            commands::ai_import::ai_usage_summary,
            commands::app_info::get_app_info,
            commands::database::switch_active_database,
            commands::notifications::get_notification_status,
            commands::notifications::set_notification_config,
            commands::notifications::test_desktop_notification,
            commands::notifications::test_ntfy_notification,
            commands::notifications::test_discord_webhook,
            // 2.113.0: the "Send to Discord" button, and what was sent.
            commands::discord::send_to_discord,
            commands::discord::get_discord_deliveries,
            commands::discord::preview_discord,
            commands::notifications::check_and_send_notifications,
            // 2.100.0: Mail Intelligence (phase 1).
            commands::mail::get_mail_status,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail::connect_mail_gmail,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail::cancel_connect_mail,
            commands::mail::disconnect_mail,
            commands::mail::set_mail_paused,
            commands::mail::set_mail_retention_days,
            commands::mail::set_mail_extra_senders,
            commands::mail_view::get_mail_view,
            commands::mail_view::read_mail_attachment,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail_view::open_mail_attachment,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail_view::save_mail_attachment,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail_view::download_mail_link,
            commands::mail::list_mail_mutes,
            commands::mail::mute_mail_sender,
            commands::mail::unmute_mail_sender,
            commands::mail::mute_mail_subject,
            commands::mail::set_mail_screen_options,
            commands::mail::set_mail_read_speed,
            commands::mail_share::mail_share_info,
            commands::mail_share::mail_share_export,
            commands::mail_share::mail_share_import,
            commands::mail_share::mail_share_cursor,
            commands::mail_share::mail_share_follow,
            commands::mail_share::set_mail_share,
            commands::mail::unmute_mail_subject,
            commands::mail::star_mail,
            commands::mail::list_mail_folder_ids,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail_cleanup::scan_promotions,
            commands::mail_cleanup::list_promo_suggestions,
            commands::mail::set_mail_skip_folders,
            commands::mail::count_mail_rule_matches,
            commands::mail_view::mail_ai,
            commands::mail::set_mail_read,
            commands::mail::set_mail_scope,
            commands::mail::list_mail_labels,
            // 2.110.0: a folder is a query - no 200-message window.
            commands::mail::list_mail_folder,
            commands::mail::get_mail_folder_counts,
            commands::mail::trash_mail,
            commands::mail::restore_mail,
            commands::mail::delete_mail_forever,
            commands::mail::set_mail_read_bulk,
            commands::mail::get_mail_proposal,
            commands::mail::create_from_mail,
            commands::mail::list_mail_sale_candidates,
            commands::mail::claim_mail_automation,
            commands::mail::set_mail_reader_options,
            commands::mail::move_mail,
            commands::mail::list_mail_local_folders,
            commands::mail::create_mail_local_folder,
            commands::mail::rename_mail_local_folder,
            commands::mail::delete_mail_local_folder,
            commands::mail::set_mail_automation,
            commands::mail::set_mail_sync_interval,
            #[cfg(not(target_arch = "wasm32"))]
            commands::mail::test_mail_connection,
            commands::mail::sync_mail_now,
            commands::mail::start_mail_reader,
            commands::mail::wake_mail_reader,
            commands::mail::hold_mail_reader,
            commands::mail::clear_stale_mail_sync,
            commands::mail::list_mail_messages,
            commands::mail::get_mail_message,
            commands::mail::set_mail_review_outcome,
            commands::mail::get_mail_overview,
            // 2.109.0: settings that live inside Mail itself - its own
            // rules, sender list and Gmail filter.
            commands::mail::list_mail_rules,
            commands::mail::save_mail_rule,
            commands::mail::delete_mail_rule,
            commands::mail::apply_mail_rules_now,
            commands::mail::list_mail_senders,
            commands::mail::save_mail_sender,
            commands::mail::delete_mail_sender,
            commands::mail::set_mail_custom_query,
            commands::settings::get_app_setting,
            commands::settings::set_app_setting,
            commands::settings::get_anthropic_api_key_configured,
            commands::settings::set_anthropic_api_key,
            commands::sheets_sync::get_sheets_connection_status,
            commands::sheets_sync::set_sheets_connection,
            commands::sheets_sync::clear_sheets_connection,
            #[cfg(not(target_arch = "wasm32"))]
            commands::sheets_sync::test_sheets_connection,
            #[cfg(not(target_arch = "wasm32"))]
            commands::sheets_sync::detect_spreadsheet_tabs,
            #[cfg(not(target_arch = "wasm32"))]
            commands::pulls_sheet_sync::sync_pulls,
            #[cfg(not(target_arch = "wasm32"))]
            commands::pulls_sheet_sync::push_pulls,
            #[cfg(not(target_arch = "wasm32"))]
            commands::pulls_sheet_sync::create_pulls_sheet,
            #[cfg(not(target_arch = "wasm32"))]
            commands::pulls_sheet_sync::setup_pulls_sheet,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::sync_orders,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::push_orders,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::sync_sales,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::push_sales,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::force_push_sales,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::create_orders_sheet,
            #[cfg(not(target_arch = "wasm32"))]
            commands::orders_sheet_sync::setup_orders_sheet,
            commands::google_auth::get_google_sign_in_status,
            #[cfg(not(target_arch = "wasm32"))]
            commands::google_auth::start_google_sign_in,
            #[cfg(not(target_arch = "wasm32"))]
            commands::google_auth::cancel_google_sign_in,
            commands::google_auth::google_sign_out,
            commands::firebase_google_auth::firebase_google_sign_in_available,
            #[cfg(not(target_arch = "wasm32"))]
            commands::firebase_google_auth::start_firebase_google_sign_in,
            #[cfg(not(target_arch = "wasm32"))]
            commands::firebase_google_auth::cancel_firebase_google_sign_in,
            commands::currency::convert_currency,
            commands::data_health::check_data_health,
            commands::notes::list_note_sheets,
            commands::notes::create_note_sheet,
            commands::notes::rename_note_sheet,
            commands::notes::delete_note_sheet,
            commands::notes::set_note_sheet_flags,
            commands::notes::add_note_column,
            commands::notes::rename_note_column,
            commands::notes::delete_note_column,
            commands::notes::reorder_note_column,
            commands::notes::ensure_note_rows,
            commands::notes::list_note_rows,
            commands::notes::create_note_row,
            commands::notes::update_note_row,
            commands::notes::delete_note_row,
            commands::notes::set_note_cell_format,
            commands::notes::set_note_cell_merge,
            commands::notes::set_note_frozen_rows,
            commands::notes::insert_note_row_at,
            commands::notes::duplicate_note_row,
            commands::notes::set_note_row_height,
            commands::notes::set_note_sheet_row_height,
            commands::notes::set_note_column_width,
            commands::notes::search_notes,
            commands::alerts::list_sheet_alerts,
            commands::alerts::save_sheet_alert,
            commands::alerts::set_sheet_alert_done,
            commands::alerts::delete_sheet_alert,
            commands::alerts::check_sheet_alerts,
            commands::notepad::list_notes,
            commands::notepad::get_note,
            commands::notepad::create_note,
            commands::notepad::update_note,
            commands::notepad::set_note_flags,
            commands::notepad::delete_note,
            commands::notepad::list_note_pages,
            commands::notepad::create_note_page,
            commands::notepad::rename_note_page,
            commands::notepad::save_note_page,
            commands::notepad::delete_note_page,
            commands::notepad::add_note_image,
            commands::notepad::list_note_images,
            commands::notepad::delete_note_image,
            commands::notepad::list_note_page_links,
            commands::notepad::reorder_note_pages,
            commands::notepad::reorder_notes,
            commands::notepad::duplicate_note_page,
            commands::notepad::duplicate_note,
            commands::notepad::set_note_image_caption,
            commands::notepad::add_note_link,
            commands::notepad::delete_note_link,
            commands::notepad::search_notepad,
            commands::workspace::list_workspace_items,
            commands::workspace::save_workspace_item,
            commands::workspace::set_workspace_item_flags,
            commands::workspace::delete_workspace_item,
            commands::workspace::duplicate_workspace_item,
            commands::workspace::search_workspace,
            commands::currency::get_preferred_currency,
            commands::currency::set_preferred_currency,
            commands::price_checker::list_marketplaces,
            commands::price_checker::create_marketplace,
            commands::price_checker::delete_marketplace,
            commands::price_checker::save_event_marketplace_link,
            commands::price_checker::save_price_check,
            commands::price_checker::get_price_checker_summary,
            commands::price_checker::list_price_checker_overview,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::open_price_scanner,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::scan_visible_prices,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::cancel_price_scan,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::export_scan_results_csv,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::close_price_scanner,
            commands::price_checker_analysis::compute_market_analysis,
            commands::price_checker_analysis::compute_comparable_market,
            #[cfg(not(target_arch = "wasm32"))]
            commands::price_checker_scanner::start_price_scan_run,
        ]
    };
}
