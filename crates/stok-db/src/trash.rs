use serde_json::json;
use sqlx::{Row, SqliteConnection};
use stok_core::catalog::sku_taken_error;
use stok_core::firm::firm_name_taken;
use stok_core::trash::*;
use stok_core::{Ctx, DomainError, Result, check_delete_confirmation};

use crate::audit::{AuditAction, write_audit};
use crate::catalog::{fetch_product, fetch_variant, sku_taken};
use crate::firms::{fetch_firm, firm_name_used};
use crate::store::{DbExt, Store, purge_guard_off, purge_guard_on};

pub(crate) async fn retention_days(conn: &mut SqliteConnection, ctx: &Ctx) -> Result<i64> {
    let v: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE tenant_id = $1 AND key = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(RETENTION_KEY)
        .fetch_optional(conn)
        .await
        .db()?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_RETENTION_DAYS))
}

async fn exec(conn: &mut SqliteConnection, sql: &str, id: &str) -> Result<u64> {
    Ok(sqlx::query(sql).bind(id).execute(conn).await.db()?.rows_affected())
}

fn stale_or_ok(row_version: i64, expected: i64) -> Result<()> {
    if row_version == expected { Ok(()) } else { Err(DomainError::Conflict) }
}

async fn restore_in(conn: &mut SqliteConnection, ctx: &Ctx, kind: TrashKind, id: &str, expected: i64) -> Result<()> {
    let now = stok_core::now_ms();
    let after = match kind {
        TrashKind::Product => {
            let p = fetch_product(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "product" })?;
            let deleted_at = p.deleted_at.ok_or_else(not_in_trash_state)?;
            stale_or_ok(p.row_version, expected)?;
            if let Some(s) = &p.sku {
                if sku_taken(&mut *conn, ctx, s, id).await? {
                    return Err(sku_taken_error());
                }
            }
            // Ürünle birlikte çöpe giden varyantlar (aynı deleted_at) geri gelir.
            let vs = sqlx::query("SELECT id, sku FROM variants WHERE product_id = $1 AND deleted_at = $2")
                .bind(id)
                .bind(deleted_at)
                .fetch_all(&mut *conn)
                .await
                .db()?;
            for v in &vs {
                let vid: String = v.try_get("id").db()?;
                if let Some(s) = v.try_get::<Option<String>, _>("sku").db()? {
                    if sku_taken(&mut *conn, ctx, &s, &vid).await? {
                        return Err(sku_taken_error());
                    }
                }
            }
            sqlx::query("UPDATE variants SET deleted_at = NULL, updated_at = $1, row_version = row_version + 1 WHERE product_id = $2 AND deleted_at = $3")
                .bind(now)
                .bind(id)
                .bind(deleted_at)
                .execute(&mut *conn)
                .await
                .db()?;
            sqlx::query("UPDATE products SET deleted_at = NULL, updated_at = $1, row_version = row_version + 1 WHERE id = $2")
                .bind(now)
                .bind(id)
                .execute(&mut *conn)
                .await
                .db()?;
            json!({ "name": p.name })
        }
        TrashKind::Variant => {
            let v = fetch_variant(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "variant" })?;
            v.deleted_at.ok_or_else(not_in_trash_state)?;
            let parent = fetch_product(&mut *conn, ctx, &v.product_id.to_string()).await?;
            if parent.as_ref().is_none_or(|p| p.deleted_at.is_some()) {
                return Err(DomainError::validation("product", "Önce ürünü geri getirin"));
            }
            stale_or_ok(v.row_version, expected)?;
            if let Some(s) = &v.sku {
                if sku_taken(&mut *conn, ctx, s, id).await? {
                    return Err(sku_taken_error());
                }
            }
            sqlx::query("UPDATE variants SET deleted_at = NULL, updated_at = $1, row_version = row_version + 1 WHERE id = $2")
                .bind(now)
                .bind(id)
                .execute(&mut *conn)
                .await
                .db()?;
            json!({ "name": v.name })
        }
        TrashKind::Firm => {
            let f = fetch_firm(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "firm" })?;
            f.deleted_at.ok_or_else(not_in_trash_state)?;
            stale_or_ok(f.row_version, expected)?;
            if firm_name_used(&mut *conn, ctx, &f.name, id).await? {
                return Err(firm_name_taken());
            }
            sqlx::query("UPDATE firms SET deleted_at = NULL, updated_at = $1, row_version = row_version + 1 WHERE id = $2")
                .bind(now)
                .bind(id)
                .execute(&mut *conn)
                .await
                .db()?;
            json!({ "name": f.name })
        }
    };
    write_audit(conn, ctx, kind.entity(), id, AuditAction::Restore, None, Some(after)).await
}

fn not_in_trash_state() -> DomainError {
    DomainError::validation("state", "Kayıt çöp kutusunda değil")
}

/// Çöpteki kaydı kalıcı siler. Çağıran transaction açar; onay kontrolü dışarıda.
async fn purge_in(conn: &mut SqliteConnection, ctx: &Ctx, kind: TrashKind, id: &str, with_history: bool) -> Result<PurgeReport> {
    let mut report = PurgeReport::default();
    // Silinecek varlıkların (entity, id) listesi ve adı.
    let (name, entities): (String, Vec<(&'static str, String)>) = match kind {
        TrashKind::Product => {
            let p = fetch_product(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "product" })?;
            if p.deleted_at.is_none() {
                return Err(not_in_trash());
            }
            let vids: Vec<String> = sqlx::query_scalar("SELECT id FROM variants WHERE product_id = $1")
                .bind(id)
                .fetch_all(&mut *conn)
                .await
                .db()?;
            let mut e = vec![("product", id.to_string())];
            e.extend(vids.into_iter().map(|v| ("variant", v)));
            (p.name, e)
        }
        TrashKind::Variant => {
            let v = fetch_variant(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "variant" })?;
            if v.deleted_at.is_none() {
                return Err(not_in_trash());
            }
            (v.name, vec![("variant", id.to_string())])
        }
        TrashKind::Firm => {
            let f = fetch_firm(&mut *conn, ctx, id).await?.ok_or(DomainError::NotFound { entity: "firm" })?;
            if f.deleted_at.is_none() {
                return Err(not_in_trash());
            }
            (f.name, vec![("firm", id.to_string())])
        }
    };

    purge_guard_on(&mut *conn).await?;
    // Hareketlerde bu varlığı gösteren sütun.
    let col = match kind {
        TrashKind::Product => "product_id",
        TrashKind::Variant => "variant_id",
        TrashKind::Firm => "firm_id",
    };
    if with_history && kind != TrashKind::Firm {
        report.removed_movements = exec(
            &mut *conn,
            &format!(
                "DELETE FROM stock_movements WHERE {col} = $1
                    OR reverses IN (SELECT id FROM stock_movements WHERE {col} = $1)"
            ),
            id,
        )
        .await? as u32;
    }
    // Kalan hareketlerde kimlik bağlantısı kaldırılır; ad kopyaları kalır.
    match kind {
        TrashKind::Product => {
            exec(&mut *conn, "UPDATE stock_movements SET product_id = NULL, variant_id = NULL WHERE product_id = $1", id).await?;
        }
        TrashKind::Variant => {
            exec(&mut *conn, "UPDATE stock_movements SET variant_id = NULL WHERE variant_id = $1", id).await?;
        }
        TrashKind::Firm => {
            exec(&mut *conn, "UPDATE stock_movements SET firm_id = NULL WHERE firm_id = $1", id).await?;
            if with_history {
                report.note = Some(FIRM_PURGE_NOTE.into());
            }
        }
    }
    for (entity, eid) in &entities {
        if with_history {
            sqlx::query("DELETE FROM audit_log WHERE tenant_id = $1 AND entity = $2 AND entity_id = $3")
                .bind(ctx.tenant_id.to_string())
                .bind(entity)
                .bind(eid)
                .execute(&mut *conn)
                .await
                .db()?;
        }
        exec(&mut *conn, "DELETE FROM external_refs WHERE entity_id = $1", eid).await?;
    }
    match kind {
        TrashKind::Product => {
            report.removed_entities += exec(&mut *conn, "DELETE FROM variants WHERE product_id = $1", id).await? as u32;
            report.removed_entities += exec(&mut *conn, "DELETE FROM products WHERE id = $1", id).await? as u32;
        }
        TrashKind::Variant => {
            report.removed_entities += exec(&mut *conn, "DELETE FROM variants WHERE id = $1", id).await? as u32;
        }
        TrashKind::Firm => {
            report.removed_entities += exec(&mut *conn, "DELETE FROM firms WHERE id = $1", id).await? as u32;
        }
    }
    purge_guard_off(&mut *conn).await?;

    let message = if with_history { format!("{name} geçmişiyle birlikte silindi") } else { format!("{name} kalıcı olarak silindi") };
    write_audit(conn, ctx, kind.entity(), id, AuditAction::Purge, Some(json!({ "name": name })), Some(json!({ "message": message }))).await?;
    Ok(report)
}

impl Store {
    /// Çöp kutusu, yeniden eskiye. Çöpteki ürünün varyantları ayrıca listelenmez.
    pub async fn list_trash(&self, ctx: &Ctx) -> Result<Vec<TrashItem>> {
        let mut conn = self.pool.acquire().await.db()?;
        let days = retention_days(&mut conn, ctx).await?;
        let rows = sqlx::query(
            "SELECT 'product' AS kind, p.id, p.name, NULL AS parent_name, p.deleted_at, p.row_version,
                    (SELECT COALESCE(SUM(v.stock_qty), 0) FROM variants v WHERE v.product_id = p.id AND v.deleted_at = p.deleted_at) AS stock_qty
             FROM products p WHERE p.tenant_id = $1 AND p.deleted_at IS NOT NULL
             UNION ALL
             SELECT 'variant', v.id, v.name, p.name, v.deleted_at, v.row_version, v.stock_qty
             FROM variants v JOIN products p ON p.id = v.product_id
             WHERE v.tenant_id = $1 AND v.deleted_at IS NOT NULL AND p.deleted_at IS NULL
             UNION ALL
             SELECT 'firm', f.id, f.name, NULL, f.deleted_at, f.row_version, 0
             FROM firms f WHERE f.tenant_id = $1 AND f.deleted_at IS NOT NULL
             ORDER BY 5 DESC, 2 DESC",
        )
        .bind(ctx.tenant_id.to_string())
        .fetch_all(&mut *conn)
        .await
        .db()?;
        rows.iter()
            .map(|r| {
                let kind = match r.try_get::<String, _>("kind").db()?.as_str() {
                    "product" => TrashKind::Product,
                    "variant" => TrashKind::Variant,
                    _ => TrashKind::Firm,
                };
                let deleted_at: i64 = r.try_get("deleted_at").db()?;
                Ok(TrashItem {
                    kind,
                    id: r.try_get("id").db()?,
                    name: r.try_get("name").db()?,
                    parent_name: r.try_get("parent_name").db()?,
                    deleted_at,
                    purge_at: purge_at(deleted_at, days),
                    stock_qty: r.try_get("stock_qty").db()?,
                    row_version: r.try_get("row_version").db()?,
                })
            })
            .collect()
    }

    pub async fn restore(&self, ctx: &Ctx, kind: TrashKind, id: &str, expected_version: i64) -> Result<()> {
        let mut tx = self.begin().await?;
        restore_in(&mut tx, ctx, kind, id, expected_version).await?;
        tx.commit().await.db()
    }

    pub async fn purge(&self, ctx: &Ctx, kind: TrashKind, id: &str, confirm: &str, with_history: bool) -> Result<PurgeReport> {
        check_delete_confirmation(confirm)?;
        let mut tx = self.begin().await?;
        let r = purge_in(&mut tx, ctx, kind, id, with_history).await?;
        tx.commit().await.db()?;
        Ok(r)
    }

    /// Saklama süresi dolan çöp kayıtlarını geçmişi koruyarak kalıcı siler.
    pub async fn purge_expired(&self, ctx: &Ctx, now_ms: i64) -> Result<u32> {
        let expired: Vec<(TrashKind, String)> = {
            let mut conn = self.pool.acquire().await.db()?;
            let days = retention_days(&mut conn, ctx).await?;
            let cutoff = now_ms - days * DAY_MS;
            let rows = sqlx::query(
                "SELECT 'product' AS kind, id FROM products WHERE tenant_id = $1 AND deleted_at IS NOT NULL AND deleted_at <= $2
                 UNION ALL
                 SELECT 'variant', v.id FROM variants v JOIN products p ON p.id = v.product_id
                  WHERE v.tenant_id = $1 AND v.deleted_at IS NOT NULL AND v.deleted_at <= $2 AND p.deleted_at IS NULL
                 UNION ALL
                 SELECT 'firm', id FROM firms WHERE tenant_id = $1 AND deleted_at IS NOT NULL AND deleted_at <= $2",
            )
            .bind(ctx.tenant_id.to_string())
            .bind(cutoff)
            .fetch_all(&mut *conn)
            .await
            .db()?;
            rows.iter()
                .map(|r| {
                    let kind = match r.try_get::<String, _>("kind").db()?.as_str() {
                        "product" => TrashKind::Product,
                        "variant" => TrashKind::Variant,
                        _ => TrashKind::Firm,
                    };
                    Ok((kind, r.try_get("id").db()?))
                })
                .collect::<Result<_>>()?
        };
        let mut n = 0;
        for (kind, id) in expired {
            let mut tx = self.begin().await?;
            purge_in(&mut tx, ctx, kind, &id, false).await?;
            tx.commit().await.db()?;
            n += 1;
        }
        Ok(n)
    }
}
