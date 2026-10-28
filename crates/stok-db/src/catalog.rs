use std::collections::HashMap;

use serde_json::json;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use stok_core::catalog::*;
use stok_core::{Ctx, DomainError, ProductId, Result, VariantId, check_delete_confirmation, normalize_tr, now_ms};

use crate::audit::{AuditAction, write_audit};
use crate::store::{DbExt, Store, get_id};

pub(crate) fn product_from_row(r: &SqliteRow) -> Result<Product> {
    Ok(Product {
        id: get_id(r, "id")?,
        name: r.try_get("name").db()?,
        sku: r.try_get("sku").db()?,
        description: r.try_get("description").db()?,
        image_path: r.try_get("image_path").db()?,
        row_version: r.try_get("row_version").db()?,
        created_at: r.try_get("created_at").db()?,
        updated_at: r.try_get("updated_at").db()?,
        deleted_at: r.try_get("deleted_at").db()?,
    })
}

pub(crate) fn variant_from_row(r: &SqliteRow) -> Result<Variant> {
    Ok(Variant {
        id: get_id(r, "id")?,
        product_id: get_id(r, "product_id")?,
        name: r.try_get("name").db()?,
        sku: r.try_get("sku").db()?,
        min_stock: r.try_get("min_stock").db()?,
        stock_qty: r.try_get("stock_qty").db()?,
        row_version: r.try_get("row_version").db()?,
        deleted_at: r.try_get("deleted_at").db()?,
    })
}

/// `%arama%` deseni; `%`, `_` ve `\` kaçırılır (`ESCAPE '\'` ile kullanılır).
pub(crate) fn like_pattern(s: &str) -> String {
    let n = normalize_tr(s).replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{n}%")
}

fn to_json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(json!(null))
}

/// SKU, çöpte olmayan ürün ve varyantlar arasında kullanılıyor mu?
pub(crate) async fn sku_taken(conn: &mut SqliteConnection, ctx: &Ctx, sku: &str, exclude_id: &str) -> Result<bool> {
    let key = normalize_tr(sku);
    let hit: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM products WHERE tenant_id = $1 AND sku_key = $2 AND deleted_at IS NULL AND id <> $3
         UNION ALL
         SELECT 1 FROM variants v JOIN products p ON p.id = v.product_id
          WHERE v.tenant_id = $1 AND v.sku_key = $2 AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND v.id <> $3
         LIMIT 1",
    )
    .bind(ctx.tenant_id.to_string())
    .bind(key)
    .bind(exclude_id)
    .fetch_optional(conn)
    .await
    .db()?;
    Ok(hit.is_some())
}

async fn ensure_sku_free(conn: &mut SqliteConnection, ctx: &Ctx, sku: &Option<String>, exclude_id: &str) -> Result<()> {
    if let Some(s) = sku {
        if sku_taken(conn, ctx, s, exclude_id).await? {
            return Err(sku_taken_error());
        }
    }
    Ok(())
}

/// Ürünü durumundan bağımsız getirir.
pub(crate) async fn fetch_product(conn: &mut SqliteConnection, ctx: &Ctx, id: &str) -> Result<Option<Product>> {
    let r = sqlx::query("SELECT * FROM products WHERE tenant_id = $1 AND id = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id)
        .fetch_optional(conn)
        .await
        .db()?;
    r.as_ref().map(product_from_row).transpose()
}

pub(crate) async fn fetch_variant(conn: &mut SqliteConnection, ctx: &Ctx, id: &str) -> Result<Option<Variant>> {
    let r = sqlx::query("SELECT * FROM variants WHERE tenant_id = $1 AND id = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id)
        .fetch_optional(conn)
        .await
        .db()?;
    r.as_ref().map(variant_from_row).transpose()
}

async fn active_product(conn: &mut SqliteConnection, ctx: &Ctx, id: ProductId) -> Result<Product> {
    match fetch_product(conn, ctx, &id.to_string()).await? {
        Some(p) if p.deleted_at.is_none() => Ok(p),
        _ => Err(DomainError::NotFound { entity: "product" }),
    }
}

/// Varyant ve ürünü çöpte değilse varyantı döndürür.
pub(crate) async fn active_variant(conn: &mut SqliteConnection, ctx: &Ctx, id: VariantId) -> Result<Variant> {
    let v = fetch_variant(conn, ctx, &id.to_string()).await?;
    match v {
        Some(v) if v.deleted_at.is_none() => {
            active_product(&mut *conn, ctx, v.product_id).await.map_err(|_| DomainError::NotFound { entity: "variant" })?;
            Ok(v)
        }
        _ => Err(DomainError::NotFound { entity: "variant" }),
    }
}

async fn insert_variant(conn: &mut SqliteConnection, ctx: &Ctx, product_id: ProductId, d: &VariantDraft) -> Result<Variant> {
    let name = clean_name(&d.name)?;
    let sku = clean_sku(d.sku.as_deref())?;
    check_min_stock(d.min_stock)?;
    let id = VariantId::new();
    ensure_sku_free(&mut *conn, ctx, &sku, &id.to_string()).await?;
    let now = now_ms();
    sqlx::query(
        "INSERT INTO variants (id, tenant_id, product_id, name, search_key, sku, sku_key, min_stock, stock_qty, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $9)",
    )
    .bind(id.to_string())
    .bind(ctx.tenant_id.to_string())
    .bind(product_id.to_string())
    .bind(&name)
    .bind(normalize_tr(&name))
    .bind(&sku)
    .bind(sku.as_deref().map(normalize_tr))
    .bind(d.min_stock)
    .bind(now)
    .execute(&mut *conn)
    .await
    .db()?;
    let v = Variant { id, product_id, name, sku, min_stock: d.min_stock, stock_qty: 0, row_version: 1, deleted_at: None };
    write_audit(conn, ctx, "variant", &id.to_string(), AuditAction::Create, None, Some(to_json(&v))).await?;
    Ok(v)
}

fn with_variants(product: Product, variants: Vec<Variant>) -> ProductWithVariants {
    let total_stock = variants.iter().map(|v| v.stock_qty).sum();
    ProductWithVariants { product, variants, total_stock }
}

impl Store {
    pub async fn create_product(&self, ctx: &Ctx, n: NewProduct) -> Result<ProductWithVariants> {
        let name = clean_name(&n.name)?;
        let sku = clean_sku(n.sku.as_deref())?;
        let description = clean_description(n.description.as_deref())?;
        let image_path = n.image_path.filter(|s| !s.trim().is_empty());
        let id = ProductId::new();
        let mut tx = self.begin().await?;
        ensure_sku_free(&mut tx, ctx, &sku, &id.to_string()).await?;
        let now = now_ms();
        sqlx::query(
            "INSERT INTO products (id, tenant_id, name, search_key, sku, sku_key, description, image_path, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
        )
        .bind(id.to_string())
        .bind(ctx.tenant_id.to_string())
        .bind(&name)
        .bind(normalize_tr(&name))
        .bind(&sku)
        .bind(sku.as_deref().map(normalize_tr))
        .bind(&description)
        .bind(&image_path)
        .bind(now)
        .execute(&mut *tx)
        .await
        .db()?;
        let product = Product { id, name, sku, description, image_path, row_version: 1, created_at: now, updated_at: now, deleted_at: None };
        write_audit(&mut tx, ctx, "product", &id.to_string(), AuditAction::Create, None, Some(to_json(&product))).await?;
        let drafts = if n.variants.is_empty() {
            vec![VariantDraft { name: DEFAULT_VARIANT_NAME.into(), sku: None, min_stock: 0 }]
        } else {
            n.variants
        };
        let mut variants = Vec::with_capacity(drafts.len());
        for d in &drafts {
            variants.push(insert_variant(&mut tx, ctx, id, d).await?);
        }
        tx.commit().await.db()?;
        Ok(with_variants(product, variants))
    }

    pub async fn update_product(&self, ctx: &Ctx, id: ProductId, patch: ProductPatch, expected_version: i64) -> Result<Product> {
        let mut tx = self.begin().await?;
        let before = active_product(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let mut after = before.clone();
        if let Some(n) = &patch.name {
            after.name = clean_name(n)?;
        }
        if let Some(s) = &patch.sku {
            after.sku = clean_sku(Some(s))?;
            ensure_sku_free(&mut tx, ctx, &after.sku, &id.to_string()).await?;
        }
        if let Some(d) = &patch.description {
            after.description = clean_description(Some(d))?;
        }
        if let Some(p) = &patch.image_path {
            after.image_path = Some(p.trim().to_string()).filter(|s| !s.is_empty());
        }
        after.row_version += 1;
        after.updated_at = now_ms();
        let res = sqlx::query(
            "UPDATE products SET name = $1, search_key = $2, sku = $3, sku_key = $4, description = $5, image_path = $6,
                    row_version = row_version + 1, updated_at = $7
             WHERE tenant_id = $8 AND id = $9 AND row_version = $10",
        )
        .bind(&after.name)
        .bind(normalize_tr(&after.name))
        .bind(&after.sku)
        .bind(after.sku.as_deref().map(normalize_tr))
        .bind(&after.description)
        .bind(&after.image_path)
        .bind(after.updated_at)
        .bind(ctx.tenant_id.to_string())
        .bind(id.to_string())
        .bind(expected_version)
        .execute(&mut *tx)
        .await
        .db()?;
        if res.rows_affected() == 0 {
            return Err(DomainError::Conflict);
        }
        write_audit(&mut tx, ctx, "product", &id.to_string(), AuditAction::Update, Some(to_json(&before)), Some(to_json(&after))).await?;
        tx.commit().await.db()?;
        Ok(after)
    }

    /// Ürünü (ve aktif varyantlarını) çöp kutusuna taşır; stoğu olsa da izinli.
    pub async fn trash_product(&self, ctx: &Ctx, id: ProductId, expected_version: i64, confirm: &str) -> Result<()> {
        check_delete_confirmation(confirm)?;
        let mut tx = self.begin().await?;
        let before = active_product(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let now = now_ms();
        sqlx::query("UPDATE products SET deleted_at = $1, updated_at = $1, row_version = row_version + 1 WHERE id = $2 AND row_version = $3")
            .bind(now)
            .bind(id.to_string())
            .bind(expected_version)
            .execute(&mut *tx)
            .await
            .db()?;
        sqlx::query("UPDATE variants SET deleted_at = $1, updated_at = $1, row_version = row_version + 1 WHERE product_id = $2 AND deleted_at IS NULL")
            .bind(now)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .db()?;
        write_audit(&mut tx, ctx, "product", &id.to_string(), AuditAction::Delete, Some(to_json(&before)), None).await?;
        tx.commit().await.db()
    }

    /// Çöpte olmayan ürünler, ada göre sıralı. Arama ürün adı, SKU ve varyant ad/SKU'sunda yapılır.
    pub async fn list_products(&self, ctx: &Ctx, q: ProductQuery) -> Result<Vec<ProductWithVariants>> {
        let pattern = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(like_pattern);
        let products = sqlx::query(
            "SELECT p.* FROM products p
             WHERE p.tenant_id = $1 AND p.deleted_at IS NULL
               AND ($2 IS NULL OR p.search_key LIKE $2 ESCAPE '\\' OR p.sku_key LIKE $2 ESCAPE '\\'
                    OR EXISTS (SELECT 1 FROM variants v WHERE v.product_id = p.id AND v.deleted_at IS NULL
                               AND (v.search_key LIKE $2 ESCAPE '\\' OR v.sku_key LIKE $2 ESCAPE '\\')))
             ORDER BY p.search_key, p.id",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .db()?;
        let variants = sqlx::query(
            "SELECT v.* FROM variants v JOIN products p ON p.id = v.product_id
             WHERE v.tenant_id = $1 AND v.deleted_at IS NULL AND p.deleted_at IS NULL ORDER BY v.id",
        )
        .bind(ctx.tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .db()?;
        let mut by_product: HashMap<ProductId, Vec<Variant>> = HashMap::new();
        for r in &variants {
            let v = variant_from_row(r)?;
            by_product.entry(v.product_id).or_default().push(v);
        }
        products
            .iter()
            .map(|r| {
                let p = product_from_row(r)?;
                let vs = by_product.remove(&p.id).unwrap_or_default();
                Ok(with_variants(p, vs))
            })
            .collect()
    }

    pub async fn get_product(&self, ctx: &Ctx, id: ProductId) -> Result<ProductWithVariants> {
        let mut conn = self.pool.acquire().await.db()?;
        let p = active_product(&mut conn, ctx, id).await?;
        let rows = sqlx::query("SELECT * FROM variants WHERE product_id = $1 AND deleted_at IS NULL ORDER BY id")
            .bind(id.to_string())
            .fetch_all(&mut *conn)
            .await
            .db()?;
        let vs = rows.iter().map(variant_from_row).collect::<Result<Vec<_>>>()?;
        Ok(with_variants(p, vs))
    }

    pub async fn get_variant(&self, ctx: &Ctx, id: VariantId) -> Result<Variant> {
        let mut conn = self.pool.acquire().await.db()?;
        active_variant(&mut conn, ctx, id).await
    }

    pub async fn create_variant(&self, ctx: &Ctx, n: NewVariant) -> Result<Variant> {
        let mut tx = self.begin().await?;
        active_product(&mut tx, ctx, n.product_id).await?;
        let v = insert_variant(&mut tx, ctx, n.product_id, &VariantDraft { name: n.name, sku: n.sku, min_stock: n.min_stock }).await?;
        tx.commit().await.db()?;
        Ok(v)
    }

    pub async fn update_variant(&self, ctx: &Ctx, id: VariantId, patch: VariantPatch, expected_version: i64) -> Result<Variant> {
        let mut tx = self.begin().await?;
        let before = active_variant(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let mut after = before.clone();
        if let Some(n) = &patch.name {
            after.name = clean_name(n)?;
        }
        if let Some(s) = &patch.sku {
            after.sku = clean_sku(Some(s))?;
            ensure_sku_free(&mut tx, ctx, &after.sku, &id.to_string()).await?;
        }
        if let Some(m) = patch.min_stock {
            check_min_stock(m)?;
            after.min_stock = m;
        }
        after.row_version += 1;
        let res = sqlx::query(
            "UPDATE variants SET name = $1, search_key = $2, sku = $3, sku_key = $4, min_stock = $5,
                    row_version = row_version + 1, updated_at = $6
             WHERE id = $7 AND row_version = $8",
        )
        .bind(&after.name)
        .bind(normalize_tr(&after.name))
        .bind(&after.sku)
        .bind(after.sku.as_deref().map(normalize_tr))
        .bind(after.min_stock)
        .bind(now_ms())
        .bind(id.to_string())
        .bind(expected_version)
        .execute(&mut *tx)
        .await
        .db()?;
        if res.rows_affected() == 0 {
            return Err(DomainError::Conflict);
        }
        write_audit(&mut tx, ctx, "variant", &id.to_string(), AuditAction::Update, Some(to_json(&before)), Some(to_json(&after))).await?;
        tx.commit().await.db()?;
        Ok(after)
    }

    pub async fn trash_variant(&self, ctx: &Ctx, id: VariantId, expected_version: i64, confirm: &str) -> Result<()> {
        check_delete_confirmation(confirm)?;
        let mut tx = self.begin().await?;
        let before = active_variant(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM variants WHERE product_id = $1 AND deleted_at IS NULL")
            .bind(before.product_id.to_string())
            .fetch_one(&mut *tx)
            .await
            .db()?;
        if active <= 1 {
            return Err(DomainError::validation("variant", "Ürünün en az bir varyantı olmalı; ürünü silin"));
        }
        let now = now_ms();
        sqlx::query("UPDATE variants SET deleted_at = $1, updated_at = $1, row_version = row_version + 1 WHERE id = $2")
            .bind(now)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .db()?;
        write_audit(&mut tx, ctx, "variant", &id.to_string(), AuditAction::Delete, Some(to_json(&before)), None).await?;
        tx.commit().await.db()
    }
}
