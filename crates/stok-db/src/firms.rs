use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use stok_core::catalog::clean_name;
use stok_core::firm::*;
use stok_core::{Ctx, DomainError, FirmId, Result, check_delete_confirmation, normalize_tr, now_ms};

use crate::audit::{AuditAction, write_audit};
use crate::catalog::like_pattern;
use crate::store::{DbExt, Store, get_id};

pub(crate) fn firm_from_row(r: &SqliteRow) -> Result<Firm> {
    Ok(Firm {
        id: get_id(r, "id")?,
        name: r.try_get("name").db()?,
        contact: r.try_get("contact").db()?,
        phone: r.try_get("phone").db()?,
        note: r.try_get("note").db()?,
        row_version: r.try_get("row_version").db()?,
        deleted_at: r.try_get("deleted_at").db()?,
    })
}

pub(crate) async fn fetch_firm(conn: &mut SqliteConnection, ctx: &Ctx, id: &str) -> Result<Option<Firm>> {
    let r = sqlx::query("SELECT * FROM firms WHERE tenant_id = $1 AND id = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id)
        .fetch_optional(conn)
        .await
        .db()?;
    r.as_ref().map(firm_from_row).transpose()
}

/// Aynı normalize adlı aktif firma var mı?
pub(crate) async fn firm_name_used(conn: &mut SqliteConnection, ctx: &Ctx, name: &str, exclude_id: &str) -> Result<bool> {
    let hit: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM firms WHERE tenant_id = $1 AND search_key = $2 AND deleted_at IS NULL AND id <> $3 LIMIT 1",
    )
    .bind(ctx.tenant_id.to_string())
    .bind(normalize_tr(name))
    .bind(exclude_id)
    .fetch_optional(conn)
    .await
    .db()?;
    Ok(hit.is_some())
}

async fn active_firm(conn: &mut SqliteConnection, ctx: &Ctx, id: FirmId) -> Result<Firm> {
    match fetch_firm(conn, ctx, &id.to_string()).await? {
        Some(f) if f.deleted_at.is_none() => Ok(f),
        _ => Err(DomainError::NotFound { entity: "firm" }),
    }
}

fn to_json(f: &Firm) -> serde_json::Value {
    serde_json::to_value(f).unwrap_or_default()
}

impl Store {
    pub async fn create_firm(&self, ctx: &Ctx, n: NewFirm) -> Result<Firm> {
        let f = Firm {
            id: FirmId::new(),
            name: clean_name(&n.name)?,
            contact: clean_contact(n.contact.as_deref())?,
            phone: clean_phone(n.phone.as_deref())?,
            note: clean_note(n.note.as_deref())?,
            row_version: 1,
            deleted_at: None,
        };
        let mut tx = self.begin().await?;
        if firm_name_used(&mut tx, ctx, &f.name, &f.id.to_string()).await? {
            return Err(firm_name_taken());
        }
        let now = now_ms();
        sqlx::query(
            "INSERT INTO firms (id, tenant_id, name, search_key, contact, phone, note, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)",
        )
        .bind(f.id.to_string())
        .bind(ctx.tenant_id.to_string())
        .bind(&f.name)
        .bind(normalize_tr(&f.name))
        .bind(&f.contact)
        .bind(&f.phone)
        .bind(&f.note)
        .bind(now)
        .execute(&mut *tx)
        .await
        .db()?;
        write_audit(&mut tx, ctx, "firm", &f.id.to_string(), AuditAction::Create, None, Some(to_json(&f))).await?;
        tx.commit().await.db()?;
        Ok(f)
    }

    pub async fn update_firm(&self, ctx: &Ctx, id: FirmId, patch: FirmPatch, expected_version: i64) -> Result<Firm> {
        let mut tx = self.begin().await?;
        let before = active_firm(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let mut after = before.clone();
        if let Some(n) = &patch.name {
            after.name = clean_name(n)?;
            if firm_name_used(&mut tx, ctx, &after.name, &id.to_string()).await? {
                return Err(firm_name_taken());
            }
        }
        if let Some(v) = &patch.contact {
            after.contact = clean_contact(Some(v))?;
        }
        if let Some(v) = &patch.phone {
            after.phone = clean_phone(Some(v))?;
        }
        if let Some(v) = &patch.note {
            after.note = clean_note(Some(v))?;
        }
        after.row_version += 1;
        let res = sqlx::query(
            "UPDATE firms SET name = $1, search_key = $2, contact = $3, phone = $4, note = $5,
                    row_version = row_version + 1, updated_at = $6
             WHERE id = $7 AND row_version = $8",
        )
        .bind(&after.name)
        .bind(normalize_tr(&after.name))
        .bind(&after.contact)
        .bind(&after.phone)
        .bind(&after.note)
        .bind(now_ms())
        .bind(id.to_string())
        .bind(expected_version)
        .execute(&mut *tx)
        .await
        .db()?;
        if res.rows_affected() == 0 {
            return Err(DomainError::Conflict);
        }
        write_audit(&mut tx, ctx, "firm", &id.to_string(), AuditAction::Update, Some(to_json(&before)), Some(to_json(&after))).await?;
        tx.commit().await.db()?;
        Ok(after)
    }

    pub async fn trash_firm(&self, ctx: &Ctx, id: FirmId, expected_version: i64, confirm: &str) -> Result<()> {
        check_delete_confirmation(confirm)?;
        let mut tx = self.begin().await?;
        let before = active_firm(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let now = now_ms();
        sqlx::query("UPDATE firms SET deleted_at = $1, updated_at = $1, row_version = row_version + 1 WHERE id = $2")
            .bind(now)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .db()?;
        write_audit(&mut tx, ctx, "firm", &id.to_string(), AuditAction::Delete, Some(to_json(&before)), None).await?;
        tx.commit().await.db()
    }

    /// Çöpte olmayan firmalar, ada göre.
    pub async fn list_firms(&self, ctx: &Ctx, search: Option<String>) -> Result<Vec<Firm>> {
        let pattern = search.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(like_pattern);
        let rows = sqlx::query(
            "SELECT * FROM firms WHERE tenant_id = $1 AND deleted_at IS NULL
               AND ($2 IS NULL OR search_key LIKE $2 ESCAPE '\\')
             ORDER BY search_key, id",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .db()?;
        rows.iter().map(firm_from_row).collect()
    }
}
