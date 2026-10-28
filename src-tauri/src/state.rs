use std::path::{Path, PathBuf};

use stok_core::user::{Permission, Role, SINGLE_USER_KEY, User, can};
use stok_core::{Ctx, DomainError, UserId, now_ms};
use stok_db::Store;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

pub const BACKUPS_TO_KEEP: usize = 14;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub db: PathBuf,
    pub backups: PathBuf,
    pub images: PathBuf,
}

impl AppPaths {
    pub fn under(dir: &Path) -> Self {
        Self { db: dir.join("stok.db"), backups: dir.join("yedekler"), images: dir.join("images") }
    }
}

pub struct AppState {
    pub store: RwLock<Store>,
    pub session: Mutex<Option<(Ctx, User)>>,
    pub paths: AppPaths,
}

impl AppState {
    /// Geçerli veritabanı (geri yüklemede değişebilir).
    pub async fn store(&self) -> Store {
        self.store.read().await.clone()
    }

    /// Oturum + yetki kontrolü. Kullanıcı her çağrıda tazelenir: pasifleştirilen
    /// ya da rolü değişen kullanıcı hemen etkilenir.
    pub async fn require(&self, p: Permission) -> ApiResult<(Ctx, Role)> {
        let Some((ctx, _)) = self.session.lock().await.clone() else {
            return Err(ApiError::unauthenticated());
        };
        match self.store().await.get_user(&ctx, ctx.user_id).await {
            Ok(u) if u.active => {
                if can(u.role, p) { Ok((ctx, u.role)) } else { Err(DomainError::Forbidden.into()) }
            }
            _ => {
                *self.session.lock().await = None;
                Err(ApiError::unauthenticated())
            }
        }
    }

    /// Tek kullanıcı modunda oturum yoksa ilk aktif yöneticiyle açar.
    pub async fn auto_login_if_single_user(&self) -> Result<(), DomainError> {
        if self.session.lock().await.is_some() {
            return Ok(());
        }
        let store = self.store().await;
        if is_single_user(&store).await? {
            if let Some(s) = store.first_active_admin().await? {
                *self.session.lock().await = Some(s);
            }
        }
        Ok(())
    }
}

/// Oturum gerektirmeyen bakım işleri için (otomatik çöp temizliği) sistem bağlamı.
pub async fn system_ctx(store: &Store) -> Result<Ctx, DomainError> {
    Ok(Ctx { tenant_id: store.default_tenant().await?, user_id: UserId(Uuid::nil()) })
}

pub async fn is_single_user(store: &Store) -> Result<bool, DomainError> {
    let ctx = system_ctx(store).await?;
    Ok(store.get_setting(&ctx, SINGLE_USER_KEY).await?.as_deref() == Some("1"))
}

pub async fn purge_expired_now(store: &Store) -> Result<u32, DomainError> {
    let ctx = system_ctx(store).await?;
    store.purge_expired(&ctx, now_ms()).await
}

/// Açılış: veritabanı → günlük yedek → süresi dolan çöp → tek kullanıcı oturumu.
pub async fn startup(paths: AppPaths) -> Result<AppState, DomainError> {
    for d in [&paths.backups, &paths.images] {
        std::fs::create_dir_all(d).map_err(|e| DomainError::Storage(e.to_string()))?;
    }
    let store = Store::open(&paths.db).await?;
    if let Err(e) = store.auto_backup(&paths.backups, BACKUPS_TO_KEEP).await {
        // Yedek alınamaması uygulamayı açmaya engel değil.
        eprintln!("otomatik yedek alınamadı: {e:?}");
    }
    purge_expired_now(&store).await?;
    let state = AppState { store: RwLock::new(store), session: Mutex::new(None), paths };
    state.auto_login_if_single_user().await?;
    Ok(state)
}
