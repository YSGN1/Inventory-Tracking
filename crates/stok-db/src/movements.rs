use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use stok_core::movement::*;
use stok_core::{Ctx, DomainError, MovementId, Result, now_ms, optional_text};

use crate::catalog::{active_variant, fetch_product, like_pattern};
use crate::store::{DbExt, Store, get_id, get_opt_id};

pub(crate) fn movement_from_row(r: &SqliteRow) -> Result<Movement> {
    Ok(Movement {
        id: get_id(r, "id")?,
        variant_id: get_opt_id(r, "variant_id")?,
        product_id: get_opt_id(r, "product_id")?,
        product_name: r.try_get("product_name").db()?,
        variant_name: r.try_get("variant_name").db()?,
        firm_id: get_opt_id(r, "firm_id")?,
        firm_name: r.try_get("firm_name").db()?,
        kind: MovementKind::parse(&r.try_get::<String, _>("kind").db()?)?,
        delta: r.try_get("delta").db()?,
        quantity: r.try_get("quantity").db()?,
        printed: r.try_get("printed").db()?,
        note: r.try_get("note").db()?,
        source: MovementSource::parse(&r.try_get::<String, _>("source").db()?)?,
        reverses: get_opt_id(r, "reverses")?,
        reversed_by: get_opt_id(r, "reversed_by")?,
        created_at: r.try_get("created_at").db()?,
        created_by: get_id(r, "created_by")?,
    })
}

async fn fetch_movement(conn: &mut SqliteConnection, ctx: &Ctx, id: MovementId) -> Result<Option<Movement>> {
    let r = sqlx::query("SELECT * FROM stock_movements WHERE tenant_id = $1 AND id = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id.to_string())
        .fetch_optional(conn)
        .await
        .db()?;
    r.as_ref().map(movement_from_row).transpose()
}

async fn insert_movement(conn: &mut SqliteConnection, ctx: &Ctx, m: &Movement, idempotency_key: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO stock_movements (id, tenant_id, variant_id, product_id, product_name, variant_name, firm_id, firm_name,
             kind, delta, quantity, printed, note, source, idempotency_key, reverses, created_at, created_by, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $17)",
    )
    .bind(m.id.to_string())
    .bind(ctx.tenant_id.to_string())
    .bind(m.variant_id.map(|x| x.to_string()))
    .bind(m.product_id.map(|x| x.to_string()))
    .bind(&m.product_name)
    .bind(&m.variant_name)
    .bind(m.firm_id.map(|x| x.to_string()))
    .bind(&m.firm_name)
    .bind(m.kind.as_str())
    .bind(m.delta)
    .bind(m.quantity)
    .bind(m.printed)
    .bind(&m.note)
    .bind(m.source.as_str())
    .bind(idempotency_key)
    .bind(m.reverses.map(|x| x.to_string()))
    .bind(m.created_at)
    .bind(m.created_by.to_string())
    .execute(conn)
    .await
    .db()?;
    Ok(())
}

async fn set_stock(conn: &mut SqliteConnection, variant_id: &str, qty: i64) -> Result<()> {
    sqlx::query("UPDATE variants SET stock_qty = $1, updated_at = $2 WHERE id = $3")
        .bind(qty)
        .bind(now_ms())
        .bind(variant_id)
        .execute(conn)
        .await
        .db()?;
    Ok(())
}

pub(crate) async fn record_in(conn: &mut SqliteConnection, ctx: &Ctx, n: NewMovement) -> Result<Movement> {
    validate(&n)?;
    let existing = sqlx::query("SELECT * FROM stock_movements WHERE tenant_id = $1 AND idempotency_key = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(&n.idempotency_key)
        .fetch_optional(&mut *conn)
        .await
        .db()?;
    if let Some(r) = existing {
        return movement_from_row(&r);
    }
    let variant = active_variant(&mut *conn, ctx, n.variant_id).await?;
    let product = fetch_product(&mut *conn, ctx, &variant.product_id.to_string())
        .await?
        .ok_or(DomainError::NotFound { entity: "product" })?;
    let firm_name = match n.firm_id {
        None => None,
        Some(f) => Some(
            sqlx::query_scalar::<_, String>("SELECT name FROM firms WHERE tenant_id = $1 AND id = $2 AND deleted_at IS NULL")
                .bind(ctx.tenant_id.to_string())
                .bind(f.to_string())
                .fetch_optional(&mut *conn)
                .await
                .db()?
                .ok_or(DomainError::NotFound { entity: "firm" })?,
        ),
    };
    let delta = compute_delta(n.kind, n.quantity, variant.stock_qty)?;
    let next = apply_delta(variant.stock_qty, delta)?;
    let m = Movement {
        id: MovementId::new(),
        variant_id: Some(variant.id),
        product_id: Some(product.id),
        product_name: product.name,
        variant_name: variant.name,
        firm_id: n.firm_id,
        firm_name,
        kind: n.kind,
        delta,
        quantity: n.quantity,
        printed: n.printed,
        note: optional_text("note", "Not", n.note.as_deref(), NOTE_MAX)?,
        source: n.source,
        reverses: None,
        reversed_by: None,
        created_at: now_ms(),
        created_by: ctx.user_id,
    };
    set_stock(&mut *conn, &variant.id.to_string(), next).await?;
    insert_movement(conn, ctx, &m, &n.idempotency_key).await?;
    Ok(m)
}

async fn reverse_in(conn: &mut SqliteConnection, ctx: &Ctx, id: MovementId, reason: &str) -> Result<Movement> {
    let reason = optional_text("reason", "Sebep", Some(reason), NOTE_MAX)?
        .ok_or_else(|| DomainError::validation("reason", "Geri alma sebebi yazılmalı"))?;
    let orig = fetch_movement(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "movement" })?;
    if orig.kind == MovementKind::Reversal {
        return Err(DomainError::validation("movement", "Geri alma kaydı geri alınamaz"));
    }
    if orig.reversed_by.is_some() {
        return Err(DomainError::validation("movement", "Bu hareket zaten geri alınmış"));
    }
    let variant_id = orig.variant_id.ok_or(DomainError::NotFound { entity: "variant" })?;
    let variant = active_variant(&mut *conn, ctx, variant_id).await?;
    let product = fetch_product(&mut *conn, ctx, &variant.product_id.to_string())
        .await?
        .ok_or(DomainError::NotFound { entity: "product" })?;
    let next = apply_delta(variant.stock_qty, -orig.delta)?;
    let r = Movement {
        id: MovementId::new(),
        variant_id: Some(variant.id),
        product_id: Some(product.id),
        product_name: product.name,
        variant_name: variant.name,
        firm_id: orig.firm_id,
        firm_name: orig.firm_name.clone(),
        kind: MovementKind::Reversal,
        delta: -orig.delta,
        quantity: orig.quantity,
        printed: None,
        note: Some(reason),
        source: MovementSource::Local,
        reverses: Some(orig.id),
        reversed_by: None,
        created_at: now_ms(),
        created_by: ctx.user_id,
    };
    set_stock(&mut *conn, &variant.id.to_string(), next).await?;
    insert_movement(&mut *conn, ctx, &r, &format!("reversal:{}", orig.id)).await?;
    sqlx::query("UPDATE stock_movements SET reversed_by = $1 WHERE id = $2")
        .bind(r.id.to_string())
        .bind(orig.id.to_string())
        .execute(&mut *conn)
        .await
        .db()?;
    Ok(r)
}

impl Store {
    /// Hareketi tek transaction'da kaydeder; aynı `idempotency_key` ikinci kez stoğu değiştirmez.
    pub async fn record_movement(&self, ctx: &Ctx, n: NewMovement) -> Result<Movement> {
        let mut tx = self.begin().await?;
        let m = record_in(&mut tx, ctx, n).await?;
        tx.commit().await.db()?;
        Ok(m)
    }

    /// Ters kayıtla geri alır; orijinal kayıt silinmez.
    pub async fn reverse_movement(&self, ctx: &Ctx, id: MovementId, reason: String) -> Result<Movement> {
        let mut tx = self.begin().await?;
        let r = reverse_in(&mut tx, ctx, id, &reason).await?;
        tx.commit().await.db()?;
        Ok(r)
    }

    /// Ters kayıt + yeni kayıt, tek transaction.
    pub async fn correct_movement(&self, ctx: &Ctx, id: MovementId, n: NewMovement) -> Result<(Movement, Movement)> {
        let mut tx = self.begin().await?;
        let r = reverse_in(&mut tx, ctx, id, "Düzeltme").await?;
        let m = record_in(&mut tx, ctx, n).await?;
        tx.commit().await.db()?;
        Ok((r, m))
    }

    pub async fn list_movements(&self, ctx: &Ctx, q: MovementQuery) -> Result<Vec<MovementView>> {
        let limit = if q.limit == 0 { DEFAULT_PAGE } else { q.limit.min(1000) };
        let rows = sqlx::query(
            "SELECT * FROM (
                SELECT m.*, u.display_name AS user_name,
                       CASE WHEN m.variant_id IS NULL OR v.id IS NULL OR v.deleted_at IS NOT NULL
                                 OR p.id IS NULL OR p.deleted_at IS NOT NULL
                            THEN 1 ELSE 0 END AS entity_deleted,
                       CASE WHEN m.firm_name IS NOT NULL AND (f.id IS NULL OR f.deleted_at IS NOT NULL)
                            THEN 1 ELSE 0 END AS firm_deleted
                FROM stock_movements m
                LEFT JOIN variants v ON v.id = m.variant_id
                LEFT JOIN products p ON p.id = m.product_id
                LEFT JOIN firms f ON f.id = m.firm_id
                LEFT JOIN users u ON u.id = m.created_by
                WHERE m.tenant_id = $1
             ) x
             WHERE ($2 IS NULL OR x.variant_id = $2)
               AND ($3 IS NULL OR x.product_id = $3)
               AND ($4 IS NULL OR x.firm_id = $4)
               AND ($5 IS NULL OR x.kind = $5)
               AND ($6 IS NULL OR x.created_at >= $6)
               AND ($7 IS NULL OR x.created_at <= $7)
               AND ($8 = 1 OR (x.entity_deleted = 0 AND x.firm_deleted = 0))
               AND ($9 IS NULL OR x.id < $9)
             ORDER BY x.id DESC LIMIT $10",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(q.variant_id.map(|x| x.to_string()))
        .bind(q.product_id.map(|x| x.to_string()))
        .bind(q.firm_id.map(|x| x.to_string()))
        .bind(q.kind.map(|k| k.as_str()))
        .bind(q.from_ms)
        .bind(q.to_ms)
        .bind(q.include_deleted)
        .bind(q.before.map(|x| x.to_string()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .db()?;
        rows.iter()
            .map(|r| {
                Ok(MovementView {
                    movement: movement_from_row(r)?,
                    user_name: r.try_get("user_name").db()?,
                    entity_deleted: r.try_get::<i64, _>("entity_deleted").db()? == 1,
                    firm_deleted: r.try_get::<i64, _>("firm_deleted").db()? == 1,
                })
            })
            .collect()
    }

    /// Ana ekran özeti: varyant başına Giren / Çıkan / Düzeltme / Mevcut. Geri almalar
    /// ters çevirdikleri türün sütununa eksi olarak yazılır.
    pub async fn stock_summary(&self, ctx: &Ctx, q: StockSummaryQuery) -> Result<Vec<StockSummaryRow>> {
        let pattern = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(like_pattern);
        let rows = sqlx::query(
            "SELECT v.id AS variant_id, p.id AS product_id, p.name AS product_name, v.name AS variant_name,
                    v.stock_qty AS current, v.min_stock AS min_stock,
                    COALESCE(SUM(CASE WHEN COALESCE(o.kind, m.kind) = 'in' THEN m.delta END), 0) AS total_in,
                    COALESCE(SUM(CASE WHEN COALESCE(o.kind, m.kind) = 'out' THEN -m.delta END), 0) AS total_out,
                    COALESCE(SUM(CASE WHEN COALESCE(o.kind, m.kind) = 'adjust' THEN m.delta END), 0) AS total_adjust
             FROM variants v
             JOIN products p ON p.id = v.product_id
             LEFT JOIN stock_movements m ON m.variant_id = v.id
                  AND ($2 IS NULL OR m.created_at >= $2) AND ($3 IS NULL OR m.created_at <= $3)
             LEFT JOIN stock_movements o ON o.id = m.reverses
             WHERE v.tenant_id = $1 AND v.deleted_at IS NULL AND p.deleted_at IS NULL
               AND ($4 IS NULL OR p.search_key LIKE $4 ESCAPE '\\' OR v.search_key LIKE $4 ESCAPE '\\'
                    OR p.sku_key LIKE $4 ESCAPE '\\' OR v.sku_key LIKE $4 ESCAPE '\\')
             GROUP BY v.id, p.id, p.name, v.name, v.stock_qty, v.min_stock, p.search_key
             ORDER BY p.search_key, p.id, v.id",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(q.from_ms)
        .bind(q.to_ms)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .db()?;
        rows.iter()
            .map(|r| {
                Ok(StockSummaryRow {
                    variant_id: get_id(r, "variant_id")?,
                    product_id: get_id(r, "product_id")?,
                    product_name: r.try_get("product_name").db()?,
                    variant_name: r.try_get("variant_name").db()?,
                    total_in: r.try_get("total_in").db()?,
                    total_out: r.try_get("total_out").db()?,
                    total_adjust: r.try_get("total_adjust").db()?,
                    current: r.try_get("current").db()?,
                    min_stock: r.try_get("min_stock").db()?,
                })
            })
            .collect()
    }
}
