use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::DomainError;

pub const DAY_MS: i64 = 86_400_000;
pub const DEFAULT_RETENTION_DAYS: i64 = 30;
pub const RETENTION_KEY: &str = "trash_retention_days";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TrashKind {
    Product,
    Variant,
    Firm,
}

impl TrashKind {
    /// `audit_log.entity` değeri.
    pub fn entity(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Variant => "variant",
            Self::Firm => "firm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TrashItem {
    pub kind: TrashKind,
    pub id: String,
    pub name: String,
    pub parent_name: Option<String>,
    pub deleted_at: i64,
    pub purge_at: i64,
    pub stock_qty: i64,
    pub row_version: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PurgeReport {
    pub removed_entities: u32,
    pub removed_movements: u32,
    pub note: Option<String>,
}

pub fn validate_retention_days(d: i64) -> Result<(), DomainError> {
    if (1..=365).contains(&d) {
        Ok(())
    } else {
        Err(DomainError::validation("trash_retention_days", "Çöp kutusu süresi 1 ile 365 gün arasında olmalı"))
    }
}

pub fn purge_at(deleted_at: i64, retention_days: i64) -> i64 {
    deleted_at + retention_days * DAY_MS
}

pub fn not_in_trash() -> DomainError {
    DomainError::validation("state", "Önce çöp kutusuna taşıyın")
}

pub const FIRM_PURGE_NOTE: &str =
    "Firmaya yapılan çıkışlar stoğu etkilediği için silinmedi; yalnız firma bağlantısı kaldırıldı.";
