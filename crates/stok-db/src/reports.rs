use sqlx::Row;
use stok_core::report::*;
use stok_core::{Ctx, Result};

use crate::store::{DbExt, Store, get_id, get_opt_id};

impl Store {
    /// `stock_qty <= min_stock` ve `min_stock > 0` olan aktif varyantlar, en kritik önce.
    pub async fn critical_stock(&self, ctx: &Ctx) -> Result<Vec<CriticalItem>> {
        let rows = sqlx::query(
            "SELECT v.id, p.name AS product_name, v.name AS variant_name, v.stock_qty, v.min_stock
             FROM variants v JOIN products p ON p.id = v.product_id
             WHERE v.tenant_id = $1 AND v.deleted_at IS NULL AND p.deleted_at IS NULL
               AND v.min_stock > 0 AND v.stock_qty <= v.min_stock
             ORDER BY v.stock_qty - v.min_stock, p.search_key, v.id",
        )
        .bind(ctx.tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .db()?;
        rows.iter()
            .map(|r| {
                Ok(CriticalItem {
                    variant_id: get_id(r, "id")?,
                    product_name: r.try_get("product_name").db()?,
                    variant_name: r.try_get("variant_name").db()?,
                    stock_qty: r.try_get("stock_qty").db()?,
                    min_stock: r.try_get("min_stock").db()?,
                })
            })
            .collect()
    }

    /// Firmaya yapılan, geri alınmamış çıkışlar + firma bazında toplamlar.
    pub async fn firm_delivery_report(&self, ctx: &Ctx, q: DeliveryQuery) -> Result<(Vec<FirmDeliveryRow>, Vec<FirmDeliveryTotal>)> {
        let rows = sqlx::query(
            "SELECT * FROM (
                SELECT m.*,
                       CASE WHEN m.variant_id IS NULL OR v.id IS NULL OR v.deleted_at IS NOT NULL
                                 OR p.id IS NULL OR p.deleted_at IS NOT NULL THEN 1 ELSE 0 END AS entity_deleted,
                       CASE WHEN f.id IS NULL OR f.deleted_at IS NOT NULL THEN 1 ELSE 0 END AS firm_deleted
                FROM stock_movements m
                LEFT JOIN variants v ON v.id = m.variant_id
                LEFT JOIN products p ON p.id = m.product_id
                LEFT JOIN firms f ON f.id = m.firm_id
                WHERE m.tenant_id = $1 AND m.kind = 'out' AND m.reversed_by IS NULL AND m.firm_name IS NOT NULL
             ) x
             WHERE ($2 IS NULL OR x.firm_id = $2)
               AND ($3 IS NULL OR x.created_at >= $3)
               AND ($4 IS NULL OR x.created_at <= $4)
               AND ($5 IS NULL OR COALESCE(x.printed, 0) = $5)
               AND ($6 = 1 OR (x.entity_deleted = 0 AND x.firm_deleted = 0))
             ORDER BY x.id DESC",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(q.firm_id.map(|x| x.to_string()))
        .bind(q.from_ms)
        .bind(q.to_ms)
        .bind(q.printed)
        .bind(q.include_deleted)
        .fetch_all(&self.pool)
        .await
        .db()?;
        let rows = rows
            .iter()
            .map(|r| {
                Ok(FirmDeliveryRow {
                    firm_id: get_opt_id(r, "firm_id")?,
                    firm_name: r.try_get("firm_name").db()?,
                    product_name: r.try_get("product_name").db()?,
                    variant_name: r.try_get("variant_name").db()?,
                    quantity: r.try_get("quantity").db()?,
                    printed: r.try_get::<Option<bool>, _>("printed").db()?.unwrap_or(false),
                    date_ms: r.try_get("created_at").db()?,
                    movement_id: get_id(r, "id")?,
                    entity_deleted: r.try_get::<i64, _>("entity_deleted").db()? == 1,
                    firm_deleted: r.try_get::<i64, _>("firm_deleted").db()? == 1,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let totals = delivery_totals(&rows);
        Ok((rows, totals))
    }
}
