use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    Row, Sqlite, SqliteConnection, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow},
};
use stok_core::{DomainError, Result, TenantId, now_ms};

pub const APPLICATION_ID: i64 = 0x53544B31;

pub(crate) static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct Store {
    pub(crate) pool: SqlitePool,
}

pub(crate) trait DbExt<T> {
    fn db(self) -> Result<T>;
}
impl<T> DbExt<T> for std::result::Result<T, sqlx::Error> {
    fn db(self) -> Result<T> {
        self.map_err(|e| DomainError::Storage(e.to_string()))
    }
}

/// Metin sütunundaki kimliği tipli kimliğe çevirir.
pub(crate) fn parse_id<T: FromStr>(s: &str) -> Result<T> {
    s.parse().map_err(|_| DomainError::Storage(format!("geçersiz kimlik: {s}")))
}
pub(crate) fn get_id<T: FromStr>(row: &SqliteRow, col: &str) -> Result<T> {
    let s: String = row.try_get(col).db()?;
    parse_id(&s)
}
pub(crate) fn get_opt_id<T: FromStr>(row: &SqliteRow, col: &str) -> Result<Option<T>> {
    let s: Option<String> = row.try_get(col).db()?;
    s.as_deref().map(parse_id).transpose()
}

pub(crate) type Tx = Transaction<'static, Sqlite>;

impl Store {
    pub async fn open(path: &Path) -> Result<Store> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_millis(5000));
        let pool = SqlitePoolOptions::new().max_connections(8).connect_with(opts).await.db()?;
        Self::init(pool).await
    }

    pub async fn open_in_memory() -> Result<Store> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .db()?
            .foreign_keys(true)
            .busy_timeout(Duration::from_millis(5000));
        // Bellek içi veritabanı bağlantıya özeldir: tek, hiç kapanmayan bağlantı.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(opts)
            .await
            .db()?;
        Self::init(pool).await
    }

    async fn init(pool: SqlitePool) -> Result<Store> {
        let app_id: i64 = sqlx::query_scalar("PRAGMA application_id").fetch_one(&pool).await.db()?;
        if app_id != APPLICATION_ID {
            let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'")
                .fetch_one(&pool)
                .await
                .db()?;
            if app_id != 0 || tables > 0 {
                pool.close().await;
                return Err(DomainError::validation("path", "Bu dosya geçerli bir stok veritabanı değil"));
            }
            sqlx::query(&format!("PRAGMA application_id = {APPLICATION_ID}")).execute(&pool).await.db()?;
        }
        MIGRATOR.run(&pool).await.map_err(|e| DomainError::Storage(e.to_string()))?;
        let store = Store { pool };
        store.ensure_default_tenant().await?;
        Ok(store)
    }

    async fn ensure_default_tenant(&self) -> Result<()> {
        let exists: Option<String> = sqlx::query_scalar("SELECT id FROM tenants ORDER BY id LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .db()?;
        if exists.is_none() {
            let id = TenantId::new().to_string();
            sqlx::query("INSERT INTO tenants (id, tenant_id, name, created_at, updated_at) VALUES ($1, $1, 'Varsayılan', $2, $2)")
                .bind(&id)
                .bind(now_ms())
                .execute(&self.pool)
                .await
                .db()?;
        }
        Ok(())
    }

    /// Yerel kurulumun tek kiracısı.
    pub async fn default_tenant(&self) -> Result<TenantId> {
        let id: String = sqlx::query_scalar("SELECT id FROM tenants ORDER BY id LIMIT 1")
            .fetch_one(&self.pool)
            .await
            .db()?;
        parse_id(&id)
    }

    /// Yazma transaction'ı; kilit hemen alınır, okuma-yazma yarışı olmaz.
    pub(crate) async fn begin(&self) -> Result<Tx> {
        self.pool.begin_with("BEGIN IMMEDIATE").await.db()
    }

    pub async fn pragma_i64(&self, name: &str) -> Result<i64> {
        sqlx::query_scalar(&format!("PRAGMA {name}")).fetch_one(&self.pool).await.db()
    }

    pub async fn pragma_str(&self, name: &str) -> Result<String> {
        sqlx::query_scalar(&format!("PRAGMA {name}")).fetch_one(&self.pool).await.db()
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Defter/denetim tablolarındaki silme korumasını bu transaction için açar.
pub(crate) async fn purge_guard_on(conn: &mut SqliteConnection) -> Result<()> {
    sqlx::query("INSERT INTO purge_guard (id) VALUES (1)").execute(conn).await.db()?;
    Ok(())
}
pub(crate) async fn purge_guard_off(conn: &mut SqliteConnection) -> Result<()> {
    sqlx::query("DELETE FROM purge_guard").execute(conn).await.db()?;
    Ok(())
}

#[cfg(feature = "test-util")]
impl Store {
    pub async fn raw_exec_for_test(&self, sql: &str) -> Result<u64> {
        Ok(sqlx::query(sql).execute(&self.pool).await.db()?.rows_affected())
    }
    pub async fn count_for_test(&self, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}")).fetch_one(&self.pool).await.unwrap()
    }
    pub async fn default_tenant_for_test(&self) -> String {
        self.default_tenant().await.unwrap().to_string()
    }
    pub async fn delete_all_movements_with_guard_for_test(&self) -> Result<()> {
        let mut tx = self.begin().await?;
        purge_guard_on(&mut tx).await?;
        sqlx::query("DELETE FROM stock_movements").execute(&mut *tx).await.db()?;
        purge_guard_off(&mut tx).await?;
        tx.commit().await.db()
    }
    /// Varsayılan kiracı + bir yönetici; testler için hazır `Ctx`.
    pub async fn seed_ctx_for_test(&self) -> stok_core::Ctx {
        let tenant_id = self.default_tenant().await.unwrap();
        let user_id = stok_core::UserId::new();
        sqlx::query(
            "INSERT INTO users (id, tenant_id, username, username_key, display_name, password_hash, role, created_at, updated_at)
             VALUES ($1, $2, $3, $3, 'Test Yönetici', 'x', 'admin', 0, 0)",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .bind(format!("test-{user_id}"))
        .execute(&self.pool)
        .await
        .unwrap();
        stok_core::Ctx { tenant_id, user_id }
    }
}
