pub mod commands;
pub mod error;
pub mod state;
pub mod visibility;

use std::time::Duration;

use tauri::Manager;

use crate::commands::*;
use crate::state::{AppPaths, AppState, purge_expired_now, startup};

const PURGE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let state = tauri::async_runtime::block_on(async {
                let s = startup(AppPaths::under(&dir)).await?;
                s.prepare_single_user().await?;
                Ok::<_, stok_core::DomainError>(s)
            })
            .map_err(|e| format!("Veritabanı açılamadı: {}", e.user_message()))?;
            app.manage(state);
            // Uygulama açıkken süresi dolan çöp kayıtları düzenli temizlenir.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(PURGE_INTERVAL).await;
                    let store = handle.state::<AppState>().store().await;
                    if let Err(e) = purge_expired_now(&store).await {
                        eprintln!("çöp temizliği başarısız: {e:?}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::app_status,
            auth::setup_first_admin,
            auth::login,
            auth::logout,
            auth::list_users,
            auth::create_user,
            auth::set_user_role,
            auth::set_user_active,
            auth::set_password,
            catalog::list_products,
            catalog::get_product,
            catalog::create_product,
            catalog::update_product,
            catalog::trash_product,
            catalog::create_variant,
            catalog::update_variant,
            catalog::trash_variant,
            catalog::set_product_image,
            catalog::get_product_image,
            catalog::audit_history,
            movements::record_movement,
            movements::reverse_movement,
            movements::correct_movement,
            movements::list_movements,
            movements::stock_summary,
            firms::list_firms,
            firms::create_firm,
            firms::update_firm,
            firms::trash_firm,
            reports::critical_stock,
            reports::firm_delivery_report,
            reports::export_delivery_csv,
            trash::list_trash,
            trash::restore_item,
            trash::purge_item,
            settings::get_settings,
            settings::set_single_user_mode,
            settings::set_trash_retention_days,
            settings::backup_now,
            settings::restore_backup,
            settings::open_backups_dir,
            update::check_update,
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("Stok Takip başlatılamadı");
}
