use crate::error::DomainError;

/// Türkçe kurallarla küçük harfe çevirir (İ→i, I→ı) ve kırpar.
pub fn normalize_tr(s: &str) -> String {
    s.trim().replace('İ', "i").replace('I', "ı").to_lowercase()
}

pub const DELETE_CONFIRM_WORD: &str = "SİL";

pub fn check_delete_confirmation(input: &str) -> Result<(), DomainError> {
    if input.trim() == DELETE_CONFIRM_WORD { Ok(()) }
    else { Err(DomainError::validation("confirm", "Onaylamak için SİL yazın")) }
}

/// 1..=max karakterlik zorunlu metin alanı.
pub fn require_text(field: &'static str, label: &str, v: &str, max: usize) -> Result<String, DomainError> {
    let t = v.trim();
    if t.is_empty() { return Err(DomainError::validation(field, format!("{label} boş olamaz"))); }
    if t.chars().count() > max { return Err(DomainError::validation(field, format!("{label} en fazla {max} karakter olabilir"))); }
    Ok(t.to_string())
}

/// Boş metni None yapar, uzunluğu sınırlar.
pub fn optional_text(field: &'static str, label: &str, v: Option<&str>, max: usize) -> Result<Option<String>, DomainError> {
    match v.map(str::trim) {
        None | Some("") => Ok(None),
        Some(t) if t.chars().count() > max => Err(DomainError::validation(field, format!("{label} en fazla {max} karakter olabilir"))),
        Some(t) => Ok(Some(t.to_string())),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn normalize_tr_handles_dotted_i() {
        assert_eq!(normalize_tr("İPEK Iğdır"), "ipek ığdır");
        assert_ne!(normalize_tr("ISPANAK"), normalize_tr("ispanak"));
        assert_eq!(normalize_tr("  Çanta  "), "çanta");
    }
    #[test] fn delete_confirmation_requires_exact_word() {
        assert!(check_delete_confirmation("SİL").is_ok());
        assert!(check_delete_confirmation(" SİL ").is_ok());
        for bad in ["sil", "SIL", "Sil", "", "SİLL"] {
            assert!(matches!(check_delete_confirmation(bad), Err(DomainError::Validation{field:"confirm",..})));
        }
    }
}
