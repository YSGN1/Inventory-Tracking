use std::path::Path;

use base64::Engine;
use stok_core::catalog::*;
use stok_core::user::Permission;
use stok_core::{DomainError, ProductId, VariantId};
use stok_db::AuditEntry;
use tauri::State;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub const IMAGE_MAX_BYTES: u64 = 5 * 1024 * 1024;

#[tauri::command]
pub async fn list_products(state: State<'_, AppState>, query: ProductQuery) -> ApiResult<Vec<ProductWithVariants>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.list_products(&ctx, query).await?)
}

#[tauri::command]
pub async fn get_product(state: State<'_, AppState>, id: ProductId) -> ApiResult<ProductWithVariants> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.get_product(&ctx, id).await?)
}

#[tauri::command]
pub async fn create_product(state: State<'_, AppState>, product: NewProduct) -> ApiResult<ProductWithVariants> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    // Görsel yalnız set_product_image ile eklenir.
    let product = NewProduct { image_path: None, ..product };
    Ok(state.store().await.create_product(&ctx, product).await?)
}

#[tauri::command]
pub async fn update_product(state: State<'_, AppState>, id: ProductId, patch: ProductPatch, expected_version: i64) -> ApiResult<Product> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    // İstemci görsel yolunu yalnız temizleyebilir; dosya yolu atayamaz.
    let patch = ProductPatch { image_path: patch.image_path.filter(|p| p.trim().is_empty()), ..patch };
    Ok(state.store().await.update_product(&ctx, id, patch, expected_version).await?)
}

#[tauri::command]
pub async fn trash_product(state: State<'_, AppState>, id: ProductId, expected_version: i64, confirm: String) -> ApiResult<()> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    Ok(state.store().await.trash_product(&ctx, id, expected_version, &confirm).await?)
}

#[tauri::command]
pub async fn create_variant(state: State<'_, AppState>, variant: NewVariant) -> ApiResult<Variant> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    Ok(state.store().await.create_variant(&ctx, variant).await?)
}

#[tauri::command]
pub async fn update_variant(state: State<'_, AppState>, id: VariantId, patch: VariantPatch, expected_version: i64) -> ApiResult<Variant> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    Ok(state.store().await.update_variant(&ctx, id, patch, expected_version).await?)
}

#[tauri::command]
pub async fn trash_variant(state: State<'_, AppState>, id: VariantId, expected_version: i64, confirm: String) -> ApiResult<()> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    Ok(state.store().await.trash_variant(&ctx, id, expected_version, &confirm).await?)
}

fn image_error(msg: &str) -> ApiError {
    DomainError::validation("image", msg).into()
}

/// Uzantı + dosya imzası; yalnız png/jpg/webp.
fn image_kind(path: &Path, head: &[u8]) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let magic_ok = match ext.as_str() {
        "png" => head.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => head.starts_with(&[0xFF, 0xD8, 0xFF]),
        "webp" => head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP",
        _ => false,
    };
    match (ext.as_str(), magic_ok) {
        ("png", true) => Some("png"),
        ("jpg" | "jpeg", true) => Some("jpg"),
        ("webp", true) => Some("webp"),
        _ => None,
    }
}

fn mime(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

#[tauri::command]
pub async fn set_product_image(state: State<'_, AppState>, product_id: ProductId, source_path: String) -> ApiResult<Product> {
    let (ctx, _) = state.require(Permission::ManageCatalog).await?;
    let src = Path::new(&source_path);
    let meta = std::fs::metadata(src).map_err(|_| image_error("Dosya okunamadı"))?;
    if !meta.is_file() {
        return Err(image_error("Dosya okunamadı"));
    }
    if meta.len() > IMAGE_MAX_BYTES {
        return Err(image_error("Görsel en fazla 5 MB olabilir"));
    }
    let bytes = std::fs::read(src).map_err(ApiError::io)?;
    let ext = image_kind(src, &bytes).ok_or_else(|| image_error("Yalnız PNG, JPG veya WEBP görsel eklenebilir"))?;
    let store = state.store().await;
    let product = store.get_product(&ctx, product_id).await?.product;
    let dir = &state.paths.images;
    std::fs::create_dir_all(dir).map_err(ApiError::io)?;
    let file_name = format!("{product_id}.{ext}");
    for other in ["png", "jpg", "webp"] {
        if other != ext {
            let _ = std::fs::remove_file(dir.join(format!("{product_id}.{other}")));
        }
    }
    std::fs::write(dir.join(&file_name), &bytes).map_err(ApiError::io)?;
    let patch = ProductPatch { image_path: Some(file_name), ..Default::default() };
    Ok(store.update_product(&ctx, product_id, patch, product.row_version).await?)
}

/// Görsel `data:` URL'si olarak (uzak/asset protokolü açmadan).
#[tauri::command]
pub async fn get_product_image(state: State<'_, AppState>, product_id: ProductId) -> ApiResult<Option<String>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    let p = state.store().await.get_product(&ctx, product_id).await?.product;
    let Some(name) = p.image_path else { return Ok(None) };
    // Yalnız uygulamanın yazdığı "<id>.<ext>" adları okunur.
    let ext = name.rsplit('.').next().unwrap_or_default().to_string();
    if name != format!("{product_id}.{ext}") {
        return Ok(None);
    }
    match std::fs::read(state.paths.images.join(&name)) {
        Ok(bytes) => Ok(Some(format!("data:{};base64,{}", mime(&ext), base64::engine::general_purpose::STANDARD.encode(bytes)))),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn audit_history(state: State<'_, AppState>, entity: String, entity_id: String) -> ApiResult<Vec<AuditEntry>> {
    let (ctx, _) = state.require(Permission::ViewAll).await?;
    Ok(state.store().await.audit_history(&ctx, &entity, &entity_id).await?)
}
