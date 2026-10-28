use stok_core::MovementId;
use stok_core::movement::*;
use stok_core::user::Permission;
use tauri::State;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::visibility::{include_deleted, label_movement};

#[tauri::command]
pub async fn record_movement(state: State<'_, AppState>, movement: NewMovement) -> ApiResult<Movement> {
    let (ctx, _) = state.require(Permission::RecordMovement).await?;
    Ok(state.store().await.record_movement(&ctx, movement).await?)
}

#[tauri::command]
pub async fn reverse_movement(state: State<'_, AppState>, id: MovementId, reason: String) -> ApiResult<Movement> {
    let (ctx, _) = state.require(Permission::CorrectMovement).await?;
    Ok(state.store().await.reverse_movement(&ctx, id, reason).await?)
}

#[tauri::command]
pub async fn correct_movement(state: State<'_, AppState>, id: MovementId, movement: NewMovement) -> ApiResult<(Movement, Movement)> {
    let (ctx, _) = state.require(Permission::CorrectMovement).await?;
    Ok(state.store().await.correct_movement(&ctx, id, movement).await?)
}

#[tauri::command]
pub async fn list_movements(state: State<'_, AppState>, query: MovementQuery) -> ApiResult<Vec<MovementView>> {
    let (ctx, role) = state.require(Permission::ViewAll).await?;
    let query = MovementQuery { include_deleted: include_deleted(role), ..query };
    let rows = state.store().await.list_movements(&ctx, query).await?;
    Ok(rows.into_iter().map(label_movement).collect())
}

#[tauri::command]
pub async fn stock_summary(state: State<'_, AppState>, query: StockSummaryQuery) -> ApiResult<Vec<StockSummaryRow>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.stock_summary(&ctx, query).await?)
}
