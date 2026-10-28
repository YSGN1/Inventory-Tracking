use std::path::Path;

use serde::Serialize;
use stok_core::trash::{DEFAULT_RETENTION_DAYS, RETENTION_KEY};
use stok_core::user::{Permission, SINGLE_USER_KEY};
use stok_db::Store;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use super::auth::{AppStatus, status};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Settings {
    pub single_user_mode: bool,
    pub trash_retention_days: i64,
    pub backups_dir: String,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> ApiResult<Settings> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    let store = state.store().await;
    let days = store.get_setting(&ctx, RETENTION_KEY).await?.and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_RETENTION_DAYS);
    Ok(Settings {
        single_user_mode: store.get_setting(&ctx, SINGLE_USER_KEY).await?.as_deref() != Some("0"),
        trash_retention_days: days,
        backups_dir: state.paths.backups.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub async fn set_single_user_mode(state: State<'_, AppState>, enabled: bool) -> ApiResult<()> {
    let (ctx, _) = state.require(Permission::ManageSettings).await?;
    Ok(state.store().await.set_setting(&ctx, SINGLE_USER_KEY, if enabled { "1" } else { "0" }).await?)
}

#[tauri::command]
pub async fn set_trash_retention_days(state: State<'_, AppState>, days: i64) -> ApiResult<()> {
    let (ctx, _) = state.require(Permission::ManageSettings).await?;
    Ok(state.store().await.set_setting(&ctx, RETENTION_KEY, &days.to_string()).await?)
}

#[tauri::command]
pub async fn backup_now(state: State<'_, AppState>, dest_path: String) -> ApiResult<()> {
    state.require(Permission::ManageSettings).await?;
    Ok(state.store().await.backup_to(Path::new(&dest_path)).await?)
}

/// Yedeği geri yükler; oturum kapanır (geri yüklenen veride kullanıcılar farklı olabilir).
#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, src_path: String) -> ApiResult<AppStatus> {
    state.require(Permission::ManageSettings).await?;
    {
        let mut guard = state.store.write().await;
        let current = guard.clone();
        match current.restore_from(Path::new(&src_path), &state.paths.db).await {
            Ok(s) => *guard = s,
            Err(e) => {
                // Kopyalama aşamasında hata olduysa eski bağlantı kapanmış olabilir: yeniden aç.
                if let Ok(s) = Store::open(&state.paths.db).await {
                    *guard = s;
                }
                return Err(e.into());
            }
        }
    }
    *state.session.lock().await = None;
    status(&state).await
}

#[tauri::command]
pub async fn open_backups_dir<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> ApiResult<()> {
    state.require(Permission::ManageSettings).await?;
    app.opener()
        .open_path(state.paths.backups.to_string_lossy(), None::<&str>)
        .map_err(|e| ApiError { code: "io".into(), message: format!("Klasör açılamadı: {e}") })
}
