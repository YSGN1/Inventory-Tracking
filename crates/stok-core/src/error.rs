#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {entity}")]
    NotFound { entity: &'static str },
    #[error("validation: {field}: {message}")]
    Validation { field: &'static str, message: String },
    #[error("insufficient stock: {available} < {requested}")]
    InsufficientStock { available: i64, requested: i64 },
    #[error("conflict")]
    Conflict,
    #[error("forbidden")]
    Forbidden,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("storage: {0}")]
    Storage(String),
}

impl DomainError {
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        Self::Validation { field, message: message.into() }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::Validation { .. } => "validation",
            Self::InsufficientStock { .. } => "insufficient_stock",
            Self::Conflict => "conflict",
            Self::Forbidden => "forbidden",
            Self::InvalidCredentials => "invalid_credentials",
            Self::Storage(_) => "storage",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::NotFound { .. } => "Kayıt bulunamadı".into(),
            Self::Validation { message, .. } => message.clone(),
            Self::InsufficientStock { available, requested } =>
                format!("Yetersiz stok: mevcut {available}, istenen {requested}"),
            Self::Conflict => "Kayıt başka biri tarafından değiştirildi, lütfen yenileyin".into(),
            Self::Forbidden => "Bu işlem için yetkiniz yok".into(),
            Self::InvalidCredentials => "Kullanıcı adı veya parola hatalı".into(),
            Self::Storage(_) => "Veritabanı hatası oluştu".into(),
        }
    }
}

pub type Result<T, E = DomainError> = std::result::Result<T, E>;
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn insufficient_stock_message_is_turkish() {
        let e = DomainError::InsufficientStock{available:3, requested:5};
        assert_eq!(e.user_message(), "Yetersiz stok: mevcut 3, istenen 5");
        assert_eq!(e.code(), "insufficient_stock");
    }
}
