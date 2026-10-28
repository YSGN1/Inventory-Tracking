use stok_core::FirmId;
use stok_core::firm::*;
use stok_core::user::Permission;
use tauri::State;

use crate::error::ApiResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_firms(state: State<'_, AppState>, search: Option<String>) -> ApiResult<Vec<Firm>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.list_firms(&ctx, search).await?)
}

#[tauri::command]
pub async fn create_firm(state: State<'_, AppState>, firm: NewFirm) -> ApiResult<Firm> {
    let (ctx, _) = state.require(Permission::ManageFirms).await?;
    Ok(state.store().await.create_firm(&ctx, firm).await?)
}

#[tauri::command]
pub async fn update_firm(state: State<'_, AppState>, id: FirmId, patch: FirmPatch, expected_version: i64) -> ApiResult<Firm> {
    let (ctx, _) = state.require(Permission::ManageFirms).await?;
    Ok(state.store().await.update_firm(&ctx, id, patch, expected_version).await?)
}

#[tauri::command]
pub async fn trash_firm(state: State<'_, AppState>, id: FirmId, expected_version: i64, confirm: String) -> ApiResult<()> {
    let (ctx, _) = state.require(Permission::ManageFirms).await?;
    Ok(state.store().await.trash_firm(&ctx, id, expected_version, &confirm).await?)
}
