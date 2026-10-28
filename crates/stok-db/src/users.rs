use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use serde_json::json;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use stok_core::user::*;
use stok_core::{Ctx, DomainError, Result, UserId, normalize_tr, now_ms};
use uuid::Uuid;

use crate::audit::{AuditAction, write_audit};
use crate::store::{DbExt, Store, get_id};

fn hash_password(p: &str) -> Result<String> {
    check_password(p)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(p.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| DomainError::Storage("parola işlenemedi".into()))
}

fn verify_password(p: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(p.as_bytes(), &h).is_ok())
}

fn user_from_row(r: &SqliteRow) -> Result<User> {
    Ok(User {
        id: get_id(r, "id")?,
        username: r.try_get("username").db()?,
        display_name: r.try_get("display_name").db()?,
        role: Role::parse(&r.try_get::<String, _>("role").db()?),
        active: r.try_get("active").db()?,
        row_version: r.try_get("row_version").db()?,
    })
}

async fn fetch_user(conn: &mut SqliteConnection, ctx: &Ctx, id: UserId) -> Result<User> {
    let r = sqlx::query("SELECT * FROM users WHERE tenant_id = $1 AND id = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id.to_string())
        .fetch_optional(conn)
        .await
        .db()?
        .ok_or(DomainError::NotFound { entity: "user" })?;
    user_from_row(&r)
}

async fn other_active_admins(conn: &mut SqliteConnection, ctx: &Ctx, id: UserId) -> Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE tenant_id = $1 AND role = 'admin' AND active = 1 AND id <> $2")
        .bind(ctx.tenant_id.to_string())
        .bind(id.to_string())
        .fetch_one(conn)
        .await
        .db()
}

fn last_admin_error() -> DomainError {
    DomainError::validation("role", "En az bir aktif yönetici kalmalı")
}

async fn insert_user(conn: &mut SqliteConnection, ctx: &Ctx, id: UserId, n: NewUser) -> Result<User> {
    let username = clean_username(&n.username)?;
    let display_name = clean_display_name(&n.display_name)?;
    let hash = hash_password(&n.password)?;
    let taken: Option<i64> = sqlx::query_scalar("SELECT 1 FROM users WHERE tenant_id = $1 AND username_key = $2")
        .bind(ctx.tenant_id.to_string())
        .bind(normalize_tr(&username))
        .fetch_optional(&mut *conn)
        .await
        .db()?;
    if taken.is_some() {
        return Err(DomainError::validation("username", "Bu kullanıcı adı kullanılıyor"));
    }
    let u = User { id, username, display_name, role: n.role, active: true, row_version: 1 };
    let now = now_ms();
    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, username_key, display_name, password_hash, role, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $8)",
    )
    .bind(u.id.to_string())
    .bind(ctx.tenant_id.to_string())
    .bind(&u.username)
    .bind(normalize_tr(&u.username))
    .bind(&u.display_name)
    .bind(hash)
    .bind(u.role.as_str())
    .bind(now)
    .execute(&mut *conn)
    .await
    .db()?;
    write_audit(conn, ctx, "user", &u.id.to_string(), AuditAction::Create, None, Some(json!(u))).await?;
    Ok(u)
}

impl Store {
    pub async fn is_initialized(&self) -> Result<bool> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&self.pool).await.db()?;
        Ok(n > 0)
    }

    /// İlk kurulumda yönetici oluşturur; kullanıcı varsa `Forbidden`.
    pub async fn create_first_admin(&self, username: &str, display_name: &str, password: &str) -> Result<(Ctx, User)> {
        let tenant_id = self.default_tenant().await?;
        let mut tx = self.begin().await?;
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&mut *tx).await.db()?;
        if n > 0 {
            return Err(DomainError::Forbidden);
        }
        let id = UserId::new();
        let ctx = Ctx { tenant_id, user_id: id };
        let u = insert_user(
            &mut tx,
            &ctx,
            id,
            NewUser { username: username.into(), display_name: display_name.into(), password: password.into(), role: Role::Admin },
        )
        .await?;
        tx.commit().await.db()?;
        Ok((ctx, u))
    }

    pub async fn create_user(&self, ctx: &Ctx, n: NewUser) -> Result<User> {
        let mut tx = self.begin().await?;
        let u = insert_user(&mut tx, ctx, UserId::new(), n).await?;
        tx.commit().await.db()?;
        Ok(u)
    }

    /// Hatalı parola, olmayan veya pasif kullanıcı aynı hatayı verir.
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<(Ctx, User)> {
        let tenant_id = self.default_tenant().await?;
        let r = sqlx::query("SELECT * FROM users WHERE tenant_id = $1 AND username_key = $2")
            .bind(tenant_id.to_string())
            .bind(normalize_tr(username))
            .fetch_optional(&self.pool)
            .await
            .db()?;
        let Some(r) = r else { return Err(DomainError::InvalidCredentials) };
        let hash: String = r.try_get("password_hash").db()?;
        let u = user_from_row(&r)?;
        if !u.active || !verify_password(password, &hash) {
            return Err(DomainError::InvalidCredentials);
        }
        Ok((Ctx { tenant_id, user_id: u.id }, u))
    }

    /// Tek kullanıcı modunda otomatik oturum için ilk aktif yönetici.
    pub async fn first_active_admin(&self) -> Result<Option<(Ctx, User)>> {
        let tenant_id = self.default_tenant().await?;
        let r = sqlx::query("SELECT * FROM users WHERE tenant_id = $1 AND role = 'admin' AND active = 1 ORDER BY id LIMIT 1")
            .bind(tenant_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .db()?;
        r.map(|r| user_from_row(&r).map(|u| (Ctx { tenant_id, user_id: u.id }, u))).transpose()
    }

    pub async fn get_user(&self, ctx: &Ctx, id: UserId) -> Result<User> {
        let mut conn = self.pool.acquire().await.db()?;
        fetch_user(&mut conn, ctx, id).await
    }

    pub async fn list_users(&self, ctx: &Ctx) -> Result<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users WHERE tenant_id = $1 AND id <> $2 ORDER BY username_key")
            .bind(ctx.tenant_id.to_string())
            .bind(Uuid::nil().to_string())
            .fetch_all(&self.pool)
            .await
            .db()?;
        rows.iter().map(user_from_row).collect()
    }

    async fn update_user(
        &self,
        ctx: &Ctx,
        id: UserId,
        expected_version: i64,
        apply: impl FnOnce(&mut User) -> Result<Option<String>>,
        guard_last_admin: bool,
    ) -> Result<User> {
        let mut tx = self.begin().await?;
        let before = fetch_user(&mut tx, ctx, id).await?;
        if before.row_version != expected_version {
            return Err(DomainError::Conflict);
        }
        let mut after = before.clone();
        let new_hash = apply(&mut after)?;
        let loses_admin = before.role == Role::Admin && before.active && (after.role != Role::Admin || !after.active);
        if guard_last_admin && loses_admin && other_active_admins(&mut tx, ctx, id).await? == 0 {
            return Err(last_admin_error());
        }
        after.row_version += 1;
        let res = sqlx::query(
            "UPDATE users SET role = $1, active = $2, password_hash = COALESCE($3, password_hash),
                    row_version = row_version + 1, updated_at = $4
             WHERE id = $5 AND row_version = $6",
        )
        .bind(after.role.as_str())
        .bind(after.active)
        .bind(new_hash)
        .bind(now_ms())
        .bind(id.to_string())
        .bind(expected_version)
        .execute(&mut *tx)
        .await
        .db()?;
        if res.rows_affected() == 0 {
            return Err(DomainError::Conflict);
        }
        write_audit(&mut tx, ctx, "user", &id.to_string(), AuditAction::Update, Some(json!(before)), Some(json!(after))).await?;
        tx.commit().await.db()?;
        Ok(after)
    }

    pub async fn set_password(&self, ctx: &Ctx, id: UserId, password: &str, expected_version: i64) -> Result<User> {
        let hash = hash_password(password)?;
        self.update_user(ctx, id, expected_version, |_| Ok(Some(hash)), false).await
    }

    pub async fn set_role(&self, ctx: &Ctx, id: UserId, role: Role, expected_version: i64) -> Result<User> {
        self.update_user(ctx, id, expected_version, |u| { u.role = role; Ok(None) }, true).await
    }

    pub async fn set_active(&self, ctx: &Ctx, id: UserId, active: bool, expected_version: i64) -> Result<User> {
        self.update_user(ctx, id, expected_version, |u| { u.active = active; Ok(None) }, true).await
    }

    pub async fn get_setting(&self, ctx: &Ctx, key: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT value FROM app_settings WHERE tenant_id = $1 AND key = $2")
            .bind(ctx.tenant_id.to_string())
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .db()
    }

    pub async fn set_setting(&self, ctx: &Ctx, key: &str, value: &str) -> Result<()> {
        validate_setting(key, value)?;
        let value = value.trim();
        let mut tx = self.begin().await?;
        let before = sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE tenant_id = $1 AND key = $2")
            .bind(ctx.tenant_id.to_string())
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .db()?;
        let now = now_ms();
        sqlx::query(
            "INSERT INTO app_settings (id, tenant_id, key, value, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $5)
             ON CONFLICT (tenant_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at,
                 row_version = app_settings.row_version + 1",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(ctx.tenant_id.to_string())
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&mut *tx)
        .await
        .db()?;
        write_audit(&mut tx, ctx, "setting", key, AuditAction::Update, before.map(|b| json!(b)), Some(json!(value))).await?;
        tx.commit().await.db()
    }
}

#[cfg(feature = "test-util")]
impl Store {
    pub async fn password_hash_for_test(&self, username: &str) -> String {
        sqlx::query_scalar("SELECT password_hash FROM users WHERE username_key = $1")
            .bind(normalize_tr(username))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}
