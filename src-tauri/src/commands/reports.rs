use serde::Serialize;
use stok_core::report::*;
use stok_core::user::Permission;
use tauri::State;
use ts_rs::TS;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::visibility::{include_deleted, label_delivery};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeliveryReport {
    pub rows: Vec<FirmDeliveryRow>,
    pub totals: Vec<FirmDeliveryTotal>,
}

#[tauri::command]
pub async fn critical_stock(state: State<'_, AppState>) -> ApiResult<Vec<CriticalItem>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.critical_stock(&ctx).await?)
}

async fn delivery_rows(state: &AppState, query: DeliveryQuery) -> ApiResult<(Vec<FirmDeliveryRow>, Vec<FirmDeliveryTotal>)> {
    let (ctx, role) = state.require(Permission::ViewAll).await?;
    let query = DeliveryQuery { include_deleted: include_deleted(role), ..query };
    Ok(state.store().await.firm_delivery_report(&ctx, query).await?)
}

#[tauri::command]
pub async fn firm_delivery_report(state: State<'_, AppState>, query: DeliveryQuery) -> ApiResult<DeliveryReport> {
    let (rows, totals) = delivery_rows(&state, query).await?;
    Ok(DeliveryReport { rows: rows.into_iter().map(label_delivery).collect(), totals })
}

/// CSV'yi kullanıcının seçtiği yere yazar; satır sayısını döndürür.
#[tauri::command]
pub async fn export_delivery_csv(state: State<'_, AppState>, query: DeliveryQuery, dest_path: String) -> ApiResult<usize> {
    let (rows, _) = delivery_rows(&state, query).await?;
    // rows_to_csv etiketleri kendisi ekler; ham adlar verilir.
    std::fs::write(&dest_path, rows_to_csv(&rows)).map_err(ApiError::io)?;
    Ok(rows.len())
}
