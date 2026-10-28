use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DomainError, FirmId, optional_text};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewFirm {
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub note: Option<String>,
}

/// `None` = değişmez; `Some("")` = temizle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FirmPatch {
    pub name: Option<String>,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Firm {
    pub id: FirmId,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub note: Option<String>,
    pub row_version: i64,
    pub deleted_at: Option<i64>,
}

pub fn clean_contact(v: Option<&str>) -> Result<Option<String>, DomainError> {
    optional_text("contact", "Yetkili", v, 200)
}
pub fn clean_phone(v: Option<&str>) -> Result<Option<String>, DomainError> {
    optional_text("phone", "Telefon", v, 50)
}
pub fn clean_note(v: Option<&str>) -> Result<Option<String>, DomainError> {
    optional_text("note", "Not", v, 1000)
}
pub fn firm_name_taken() -> DomainError {
    DomainError::validation("name", "Bu adla bir firma zaten var")
}
