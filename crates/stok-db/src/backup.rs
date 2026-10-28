use std::path::{Path, PathBuf};

use sqlx::{ConnectOptions, Connection, sqlite::SqliteConnectOptions};
use stok_core::report::date_stamp_tr;
use stok_core::{DomainError, Result, now_ms};

use crate::store::{APPLICATION_ID, DbExt, Store};

fn invalid_backup() -> DomainError {
    DomainError::validation("path", "Bu dosya geçerli bir stok yedeği değil")
}

fn io(e: std::io::Error) -> DomainError {
    DomainError::Storage(e.to_string())
}

/// Dosya bu uygulamanın veritabanı mı (application_id + migration tablosu)?
pub async fn validate_backup(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(invalid_backup());
    }
    let opts = SqliteConnectOptions::new().filename(path).read_only(true).create_if_missing(false);
    let mut conn = opts.connect().await.map_err(|_| invalid_backup())?;
    let ok = async {
        let app_id: i64 = sqlx::query_scalar("PRAGMA application_id").fetch_one(&mut conn).await.ok()?;
        let has_migrations: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'")
                .fetch_optional(&mut conn)
                .await
                .ok()?;
        Some(app_id == APPLICATION_ID && has_migrations.is_some())
    }
    .await
    .unwrap_or(false);
    let _ = conn.close().await;
    if ok { Ok(()) } else { Err(invalid_backup()) }
}

/// Windows'ta kapanan bağlantının dosya tutamacı (veya virüs tarayıcısı) kısa süre kalabilir.
async fn rename_with_retry(from: &Path, to: &Path) -> Result<()> {
    let mut last = None;
    for _ in 0..50 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(io(last.expect("en az bir deneme")))
}

/// Uygulamanın ürettiği günlük yedek adı: `stok-YYYYMMDD.db`.
fn is_daily_backup_name(name: &str) -> bool {
    name.len() == "stok-YYYYMMDD.db".len()
        && name.starts_with("stok-")
        && name.ends_with(".db")
        && name[5..13].bytes().all(|b| b.is_ascii_digit())
}

impl Store {
    /// Tutarlı anlık kopya (`VACUUM INTO`). Hedef varsa reddedilir.
    pub async fn backup_to(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            return Err(DomainError::validation("path", "Bu adla bir dosya zaten var"));
        }
        let p = dest.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{p}'")).execute(&self.pool).await.db()?;
        Ok(())
    }

    /// Yedeği doğrular, mevcut dosyayı `stok.db.before-restore-<ms>` olarak saklar,
    /// yedeği yerine kopyalar ve yeniden açar.
    pub async fn restore_from(self, src: &Path, live_db: &Path) -> Result<Store> {
        validate_backup(src).await?;
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(&self.pool).await;
        self.close().await;

        let file_name = live_db.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "stok.db".into());
        let kept = live_db.with_file_name(format!("{file_name}.before-restore-{}", now_ms()));
        rename_with_retry(live_db, &kept).await?;
        for ext in ["-wal", "-shm"] {
            let side = live_db.with_file_name(format!("{file_name}{ext}"));
            if side.exists() {
                let _ = std::fs::remove_file(side);
            }
        }
        if let Err(e) = std::fs::copy(src, live_db) {
            // Kopyalanamadıysa eski veritabanına dön.
            let _ = std::fs::remove_file(live_db);
            std::fs::rename(&kept, live_db).map_err(io)?;
            Store::open(live_db).await?;
            return Err(io(e));
        }
        Store::open(live_db).await
    }

    /// Günde bir yedek; en yeni `keep` günlük yedek tutulur. Bugünün yedeği varsa `None`.
    pub async fn auto_backup(&self, dir: &Path, keep: usize) -> Result<Option<PathBuf>> {
        std::fs::create_dir_all(dir).map_err(io)?;
        let target = dir.join(format!("stok-{}.db", date_stamp_tr(now_ms())));
        if target.exists() {
            return Ok(None);
        }
        self.backup_to(&target).await?;
        let mut daily: Vec<String> = std::fs::read_dir(dir)
            .map_err(io)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| is_daily_backup_name(n))
            .collect();
        daily.sort_unstable_by(|a, b| b.cmp(a));
        for old in daily.iter().skip(keep) {
            let _ = std::fs::remove_file(dir.join(old));
        }
        Ok(Some(target))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn daily_name_filter() {
        assert!(super::is_daily_backup_name("stok-20260101.db"));
        assert!(!super::is_daily_backup_name("stok-2026010.db"));
        assert!(!super::is_daily_backup_name("benim-dosyam.db"));
        assert!(!super::is_daily_backup_name("stok-2026010a.db"));
    }
}
