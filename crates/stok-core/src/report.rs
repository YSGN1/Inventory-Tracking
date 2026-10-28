use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{FirmId, MovementId, VariantId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CriticalItem {
    pub variant_id: VariantId,
    pub product_name: String,
    pub variant_name: String,
    pub stock_qty: i64,
    pub min_stock: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FirmDeliveryRow {
    pub firm_id: Option<FirmId>,
    pub firm_name: String,
    pub product_name: String,
    pub variant_name: String,
    pub quantity: i64,
    pub printed: bool,
    pub date_ms: i64,
    pub movement_id: MovementId,
    /// Ürün veya varyant çöpte / kalıcı silinmiş.
    pub entity_deleted: bool,
    /// Firma çöpte / kalıcı silinmiş.
    pub firm_deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FirmDeliveryTotal {
    pub firm_name: String,
    pub printed_qty: i64,
    pub unprinted_qty: i64,
    pub total_qty: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeliveryQuery {
    pub firm_id: Option<FirmId>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub printed: Option<bool>,
    /// Komut katmanında rolden belirlenir; istemciden alınmaz.
    #[serde(skip)]
    #[ts(skip)]
    pub include_deleted: bool,
}

pub fn display_name(row_name: &str, entity_deleted: bool) -> String {
    if entity_deleted { format!("Silinmiş ürün: {row_name}") } else { row_name.to_string() }
}

pub fn display_firm_name(firm_name: &str, firm_deleted: bool) -> String {
    if firm_deleted { format!("Silinmiş firma: {firm_name}") } else { firm_name.to_string() }
}

/// Firma bazında toplamlar (satırların sırasıyla, ilk görülen firma önce).
pub fn delivery_totals(rows: &[FirmDeliveryRow]) -> Vec<FirmDeliveryTotal> {
    let mut out: Vec<FirmDeliveryTotal> = Vec::new();
    for r in rows {
        let name = display_firm_name(&r.firm_name, r.firm_deleted);
        let t = match out.iter_mut().position(|t| t.firm_name == name) {
            Some(i) => &mut out[i],
            None => {
                out.push(FirmDeliveryTotal { firm_name: name, printed_qty: 0, unprinted_qty: 0, total_qty: 0 });
                out.last_mut().unwrap()
            }
        };
        if r.printed { t.printed_qty += r.quantity } else { t.unprinted_qty += r.quantity }
        t.total_qty += r.quantity;
    }
    out
}

pub fn format_date_tr(ms: i64) -> String {
    chrono_tz::Europe::Istanbul
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}

/// `YYYYMMDD` (Europe/Istanbul) — günlük yedek dosya adı için.
pub fn date_stamp_tr(ms: i64) -> String {
    chrono_tz::Europe::Istanbul
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%Y%m%d").to_string())
        .unwrap_or_default()
}

/// Excel'in Türkçe karakterleri doğru açması için UTF-8 BOM + `;` ayraç.
pub fn rows_to_csv(rows: &[FirmDeliveryRow]) -> String {
    let mut w = csv::WriterBuilder::new().delimiter(b';').from_writer(Vec::new());
    let _ = w.write_record(["Tarih", "Firma", "Ürün", "Varyant", "Adet", "Baskı"]);
    for r in rows {
        let _ = w.write_record([
            format_date_tr(r.date_ms),
            display_firm_name(&r.firm_name, r.firm_deleted),
            display_name(&r.product_name, r.entity_deleted),
            r.variant_name.clone(),
            r.quantity.to_string(),
            if r.printed { "Evet" } else { "Hayır" }.to_string(),
        ]);
    }
    let body = String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default();
    format!("\u{FEFF}{body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(firm: &str, product: &str, qty: i64, printed: bool, deleted: bool) -> FirmDeliveryRow {
        FirmDeliveryRow {
            firm_id: None, firm_name: firm.into(), product_name: product.into(), variant_name: "Standart".into(),
            quantity: qty, printed, date_ms: 1_767_225_600_000, // 2026-01-01 00:00 UTC
            movement_id: MovementId::new(), entity_deleted: deleted, firm_deleted: false,
        }
    }
    #[test] fn csv_has_bom_header_and_deleted_label() {
        let csv = rows_to_csv(&[row("Öz Matbaa", "Kupa", 3, true, true)]);
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("Tarih;Firma;Ürün;Varyant;Adet;Baskı"));
        assert!(csv.contains(";Öz Matbaa;Silinmiş ürün: Kupa;") && csv.contains(";3;Evet"));
        assert!(csv.contains("01.01.2026 03:00;"), "Istanbul time: {csv}");
    }
    #[test] fn csv_quotes_separator() {
        let csv = rows_to_csv(&[row("A;B", "Kupa", 1, false, false)]);
        assert!(csv.contains(";\"A;B\";Kupa;") && csv.contains(";1;Hayır"));
    }
    #[test] fn display_name_labels() {
        assert_eq!(display_name("Kupa", false), "Kupa");
        assert_eq!(display_name("Kupa", true), "Silinmiş ürün: Kupa");
        assert_eq!(display_firm_name("ABC", true), "Silinmiş firma: ABC");
    }
}
