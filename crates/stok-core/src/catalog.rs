use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DomainError, ProductId, VariantId, optional_text, require_text};

pub const NAME_MAX: usize = 200;
pub const SKU_MAX: usize = 64;
pub const DESCRIPTION_MAX: usize = 2000;
pub const DEFAULT_VARIANT_NAME: &str = "Standart";

/// Ürünle birlikte oluşturulacak varyant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VariantDraft {
    pub name: String,
    pub sku: Option<String>,
    pub min_stock: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewProduct {
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub image_path: Option<String>,
    /// Boşsa "Standart" varyantı oluşturulur.
    #[serde(default)]
    pub variants: Vec<VariantDraft>,
}

/// `None` = değişmez; isteğe bağlı alanlarda `Some("")` = temizle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProductPatch {
    pub name: Option<String>,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewVariant {
    pub product_id: ProductId,
    pub name: String,
    pub sku: Option<String>,
    pub min_stock: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VariantPatch {
    pub name: Option<String>,
    pub sku: Option<String>,
    pub min_stock: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub image_path: Option<String>,
    pub row_version: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Variant {
    pub id: VariantId,
    pub product_id: ProductId,
    pub name: String,
    pub sku: Option<String>,
    pub min_stock: i64,
    pub stock_qty: i64,
    pub row_version: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProductWithVariants {
    pub product: Product,
    pub variants: Vec<Variant>,
    pub total_stock: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct ProductQuery {
    pub search: Option<String>,
}

pub fn clean_name(v: &str) -> Result<String, DomainError> {
    require_text("name", "Ad", v, NAME_MAX)
}

pub fn clean_sku(v: Option<&str>) -> Result<Option<String>, DomainError> {
    optional_text("sku", "SKU", v, SKU_MAX)
}

pub fn clean_description(v: Option<&str>) -> Result<Option<String>, DomainError> {
    optional_text("description", "Açıklama", v, DESCRIPTION_MAX)
}

pub fn check_min_stock(n: i64) -> Result<(), DomainError> {
    if n < 0 { Err(DomainError::validation("min_stock", "Kritik eşik negatif olamaz")) } else { Ok(()) }
}

pub fn sku_taken_error() -> DomainError {
    DomainError::validation("sku", "Bu SKU başka bir üründe kullanılıyor")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn sku_blank_is_none() {
        assert_eq!(clean_sku(Some("  ")).unwrap(), None);
        assert_eq!(clean_sku(Some(" A-1 ")).unwrap().as_deref(), Some("A-1"));
    }
    #[test] fn long_name_rejected() {
        assert!(clean_name(&"a".repeat(201)).is_err());
        assert!(check_min_stock(-1).is_err());
    }
}
