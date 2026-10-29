use serde::Serialize;
use stok_core::user::Permission;
use stok_core::{DomainError, now_ms};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use ts_rs::TS;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct UpdateInfo {
    pub current: String,
    /// Yeni sürüm numarası; güncelse `None`.
    pub available: Option<String>,
    pub notes: Option<String>,
}

fn update_err(e: tauri_plugin_updater::Error) -> ApiError {
    ApiError { code: "update".into(), message: format!("Güncelleme denetlenemedi: {e}") }
}

/// Güncelleme adresi yalnız GitHub'da derlenen sürümlere eklenir; yerel derlemede `None`.
async fn find_update<R: Runtime>(app: &AppHandle<R>) -> ApiResult<Option<Update>> {
    match app.updater() {
        Ok(u) => u.check().await.map_err(update_err),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn check_update<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> ApiResult<UpdateInfo> {
    state.require(Permission::ViewAll).await?;
    let update = find_update(&app).await?;
    Ok(UpdateInfo {
        current: app.package_info().version.to_string(),
        available: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
    })
}

/// Önce veritabanını yedekler, sonra indirip kurar ve uygulamayı yeniden başlatır.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> ApiResult<()> {
    state.require(Permission::ManageSettings).await?;
    let update = find_update(&app).await?.ok_or_else(|| DomainError::validation("update", "Yeni sürüm yok"))?;
    let backup = state.paths.backups.join(format!("stok-guncelleme-oncesi-{}.db", now_ms()));
    state.store().await.backup_to(&backup).await?;
    update.download_and_install(|_, _| {}, || {}).await.map_err(update_err)?;
    app.restart()
}
