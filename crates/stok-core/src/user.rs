use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DomainError, UserId, require_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Role {
    Admin,
    Operator,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "admin" { Self::Admin } else { Self::Operator }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Permission {
    ViewAll,
    ViewDeleted,
    RecordMovement,
    CorrectMovement,
    ManageCatalog,
    ManageFirms,
    ManageTrash,
    ManageUsers,
    ManageSettings,
}

/// Yönetici her şeyi yapar; Operatör yalnız görüntüler ve hareket girer.
pub fn can(role: Role, p: Permission) -> bool {
    match role {
        Role::Admin => true,
        Role::Operator => matches!(p, Permission::ViewAll | Permission::RecordMovement),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub active: bool,
    pub row_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewUser {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: Role,
}

pub const PASSWORD_MIN: usize = 6;

pub fn clean_username(v: &str) -> Result<String, DomainError> {
    let u = require_text("username", "Kullanıcı adı", v, 50)?;
    if u.chars().count() < 3 || u.chars().any(char::is_whitespace) {
        return Err(DomainError::validation("username", "Kullanıcı adı en az 3 karakter olmalı ve boşluk içermemeli"));
    }
    Ok(u)
}

pub fn clean_display_name(v: &str) -> Result<String, DomainError> {
    require_text("display_name", "Görünen ad", v, 100)
}

pub fn check_password(p: &str) -> Result<(), DomainError> {
    if p.chars().count() < PASSWORD_MIN {
        return Err(DomainError::validation("password", format!("Parola en az {PASSWORD_MIN} karakter olmalı")));
    }
    if p.len() > 256 {
        return Err(DomainError::validation("password", "Parola çok uzun"));
    }
    Ok(())
}

pub const SINGLE_USER_KEY: &str = "single_user_mode";

/// Ayar anahtarı ve değerini doğrular.
pub fn validate_setting(key: &str, value: &str) -> Result<(), DomainError> {
    match key {
        SINGLE_USER_KEY if value == "1" || value == "0" => Ok(()),
        SINGLE_USER_KEY => Err(DomainError::validation("single_user_mode", "Geçersiz değer")),
        crate::trash::RETENTION_KEY => {
            let d: i64 = value.trim().parse().map_err(|_| DomainError::validation("trash_retention_days", "Gün sayısı girin"))?;
            crate::trash::validate_retention_days(d)
        }
        _ => Err(DomainError::validation("key", "Bilinmeyen ayar")),
    }
}
