use serde::Serialize;
use stok_core::DomainError;
use ts_rs::TS;

/// Arayüze dönen hata: `code` makine için, `message` kullanıcıya gösterilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn unauthenticated() -> Self {
        Self { code: "unauthenticated".into(), message: "Oturum açmanız gerekiyor".into() }
    }
    pub fn io(e: std::io::Error) -> Self {
        Self { code: "io".into(), message: format!("Dosya işlemi başarısız: {e}") }
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        if let DomainError::Storage(detail) = &e {
            eprintln!("storage error: {detail}");
        }
        Self { code: e.code().into(), message: e.user_message() }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
