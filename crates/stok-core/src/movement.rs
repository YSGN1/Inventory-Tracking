use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DomainError, FirmId, MovementId, ProductId, UserId, VariantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MovementKind { In, Out, Adjust, Reversal }

impl MovementKind {
    pub fn as_str(self) -> &'static str {
        match self { Self::In => "in", Self::Out => "out", Self::Adjust => "adjust", Self::Reversal => "reversal" }
    }
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        Ok(match s {
            "in" => Self::In, "out" => Self::Out, "adjust" => Self::Adjust, "reversal" => Self::Reversal,
            _ => return Err(DomainError::Storage(format!("bilinmeyen hareket türü: {s}"))),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MovementSource { Local, Site }

impl MovementSource {
    pub fn as_str(self) -> &'static str { match self { Self::Local => "local", Self::Site => "site" } }
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s { "local" => Ok(Self::Local), "site" => Ok(Self::Site),
            _ => Err(DomainError::Storage(format!("bilinmeyen kaynak: {s}"))) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewMovement {
    pub variant_id: VariantId,
    pub kind: MovementKind,
    pub quantity: i64,
    pub firm_id: Option<FirmId>,
    pub printed: Option<bool>,
    pub note: Option<String>,
    pub idempotency_key: String,
    pub source: MovementSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Movement {
    pub id: MovementId,
    pub variant_id: Option<VariantId>,
    pub product_id: Option<ProductId>,
    pub product_name: String,
    pub variant_name: String,
    pub firm_id: Option<FirmId>,
    pub firm_name: Option<String>,
    pub kind: MovementKind,
    pub delta: i64,
    pub quantity: i64,
    pub printed: Option<bool>,
    pub note: Option<String>,
    pub source: MovementSource,
    pub reverses: Option<MovementId>,
    pub reversed_by: Option<MovementId>,
    pub created_at: i64,
    pub created_by: UserId,
}

/// Hareket + görünüm alanları. `entity_deleted`: ürün/varyant/firma çöpte veya kalıcı silinmiş.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MovementView {
    #[serde(flatten)]
    #[ts(flatten)]
    pub movement: Movement,
    pub user_name: Option<String>,
    pub entity_deleted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MovementQuery {
    pub variant_id: Option<VariantId>,
    pub product_id: Option<ProductId>,
    pub firm_id: Option<FirmId>,
    pub kind: Option<MovementKind>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    /// Komut katmanında rolden belirlenir; istemciden alınmaz.
    #[serde(skip)]
    #[ts(skip)]
    pub include_deleted: bool,
    pub limit: u32,
    pub before: Option<MovementId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StockSummaryQuery {
    pub search: Option<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StockSummaryRow {
    pub variant_id: VariantId,
    pub product_id: ProductId,
    pub product_name: String,
    pub variant_name: String,
    pub total_in: i64,
    pub total_out: i64,
    pub total_adjust: i64,
    pub current: i64,
    pub min_stock: i64,
}

pub const NOTE_MAX: usize = 500;
pub const DEFAULT_PAGE: u32 = 200;

pub fn validate(m: &NewMovement) -> Result<(), DomainError> {
    match m.kind {
        MovementKind::Reversal => return Err(DomainError::validation("kind", "Geri alma kaydı doğrudan girilemez")),
        MovementKind::In | MovementKind::Out if m.quantity <= 0 =>
            return Err(DomainError::validation("quantity", "Miktar 0'dan büyük olmalı")),
        MovementKind::Adjust if m.quantity < 0 =>
            return Err(DomainError::validation("quantity", "Sayım sonucu negatif olamaz")),
        _ => {}
    }
    if m.kind != MovementKind::Out {
        if m.printed.is_some() { return Err(DomainError::validation("printed", "Baskı bilgisi yalnız çıkışta girilir")); }
        if m.firm_id.is_some() { return Err(DomainError::validation("firm_id", "Firma yalnız çıkışta seçilir")); }
    }
    if m.idempotency_key.trim().is_empty() { return Err(DomainError::validation("idempotency_key", "İşlem anahtarı boş olamaz")); }
    if m.note.as_deref().is_some_and(|n| n.chars().count() > NOTE_MAX) {
        return Err(DomainError::validation("note", format!("Not en fazla {NOTE_MAX} karakter olabilir")));
    }
    Ok(())
}

pub fn compute_delta(kind: MovementKind, quantity: i64, current: i64) -> Result<i64, DomainError> {
    match kind {
        MovementKind::In => Ok(quantity),
        MovementKind::Out => Ok(-quantity),
        MovementKind::Adjust => Ok(quantity - current),
        MovementKind::Reversal => Err(DomainError::validation("kind", "Geri alma kaydı doğrudan girilemez")),
    }
}

pub fn apply_delta(current: i64, delta: i64) -> Result<i64, DomainError> {
    let next = current + delta;
    if next < 0 { Err(DomainError::InsufficientStock { available: current, requested: -delta }) } else { Ok(next) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn new_mv(kind: MovementKind, quantity: i64) -> NewMovement {
        NewMovement { variant_id: VariantId::new(), kind, quantity, firm_id: None, printed: None, note: None,
            idempotency_key: "k".into(), source: MovementSource::Local }
    }
    #[test] fn out_more_than_stock_fails() {
        let d = compute_delta(MovementKind::Out, 5, 3).unwrap();
        assert!(matches!(apply_delta(3, d), Err(DomainError::InsufficientStock{available:3, requested:5})));
    }
    #[test] fn adjust_sets_absolute_count() { assert_eq!(compute_delta(MovementKind::Adjust, 7, 10).unwrap(), -3); }
    #[test] fn zero_in_rejected() {
        assert!(matches!(validate(&new_mv(MovementKind::In, 0)), Err(DomainError::Validation{field:"quantity",..})));
    }
    #[test] fn printed_only_on_out() {
        let mut m = new_mv(MovementKind::In, 2); m.printed = Some(true);
        assert!(matches!(validate(&m), Err(DomainError::Validation{field:"printed",..})));
    }
    #[test] fn reversal_cannot_be_submitted() { assert!(validate(&new_mv(MovementKind::Reversal, 1)).is_err()); }
    #[test] fn out_to_exact_zero_ok() { assert_eq!(apply_delta(4, -4).unwrap(), 0); }
}
