use stok_core::trash::*;
use stok_core::user::Permission;
use tauri::State;

use crate::error::ApiResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>) -> ApiResult<Vec<TrashItem>> {
    let (ctx, _) = state.require(Permission::ManageTrash).await?;
    Ok(state.store().await.list_trash(&ctx).await?)
}

#[tauri::command]
pub async fn restore_item(state: State<'_, AppState>, kind: TrashKind, id: String, expected_version: i64) -> ApiResult<()> {
    // Yanlışlıkla silmeyi hemen "Geri al" ile düzeltmek de bu yetkiyi ister.
    let (ctx, _) = state.require(Permission::ManageTrash).await?;
    Ok(state.store().await.restore(&ctx, kind, &id, expected_version).await?)
}

#[tauri::command]
pub async fn purge_item(state: State<'_, AppState>, kind: TrashKind, id: String, confirm: String, with_history: bool) -> ApiResult<PurgeReport> {
    let (ctx, _) = state.require(Permission::ManageTrash).await?;
    Ok(state.store().await.purge(&ctx, kind, &id, &confirm, with_history).await?)
}
