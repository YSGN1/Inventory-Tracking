mod common;
use common::*;
use stok_core::*;
use stok_db::{Store, validate_backup};

async fn file_store(dir: &std::path::Path) -> (Store, Ctx, std::path::PathBuf) {
    let p = dir.join("stok.db");
    let s = Store::open(&p).await.unwrap();
    let ctx = s.seed_ctx_for_test().await;
    (s, ctx, p)
}

#[tokio::test] async fn backup_roundtrip_preserves_stock_and_trash() {
    let dir = tempfile::tempdir().unwrap();
    let (s, ctx, live) = file_store(dir.path()).await;
    let a = s.create_product(&ctx, np("Kupa")).await.unwrap();
    s.record_movement(&ctx, mv_in(a.variants[0].id, 9, "a")).await.unwrap();
    let b = s.create_product(&ctx, np("Bardak")).await.unwrap().product;
    s.trash_product(&ctx, b.id, b.row_version, "SİL").await.unwrap();
    let bak = dir.path().join("yedek.db");
    s.backup_to(&bak).await.unwrap();
    assert!(matches!(s.backup_to(&bak).await, Err(DomainError::Validation{field:"path",..})));
    validate_backup(&bak).await.unwrap();

    // change after backup, then restore → change gone
    s.record_movement(&ctx, mv_in(a.variants[0].id, 1, "b")).await.unwrap();
    let s = s.restore_from(&bak, &live).await.unwrap();
    assert_eq!(stock(&s, &ctx, a.variants[0].id).await, 9);
    assert_eq!(s.list_trash(&ctx).await.unwrap().len(), 1);
}
#[tokio::test] async fn restore_rejects_foreign_sqlite_file() {
    let dir = tempfile::tempdir().unwrap();
    let (s, _, live) = file_store(dir.path()).await;
    let foreign = dir.path().join("baska.db");
    {
        use sqlx::Connection;
        let mut c = sqlx::SqliteConnection::connect(&format!("sqlite://{}?mode=rwc", foreign.display())).await.unwrap();
        sqlx::query("CREATE TABLE x (a INTEGER)").execute(&mut c).await.unwrap();
        c.close().await.unwrap();
    }
    let text = dir.path().join("metin.db");
    std::fs::write(&text, "merhaba").unwrap();
    for f in [&foreign, &text] {
        let e = validate_backup(f).await.unwrap_err();
        assert!(matches!(&e, DomainError::Validation{field:"path", message} if message == "Bu dosya geçerli bir stok yedeği değil"), "{e:?}");
    }
    assert!(s.clone().restore_from(&foreign, &live).await.is_err());
    // store still usable after rejected restore
    assert!(s.is_initialized().await.unwrap());
}
#[tokio::test] async fn restore_keeps_previous_copy() {
    let dir = tempfile::tempdir().unwrap();
    let (s, _, live) = file_store(dir.path()).await;
    let bak = dir.path().join("yedek.db");
    s.backup_to(&bak).await.unwrap();
    let _s = s.restore_from(&bak, &live).await.unwrap();
    let kept: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("stok.db.before-restore-"))
        .collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
}
#[tokio::test] async fn auto_backup_once_per_day_and_rotates() {
    let dir = tempfile::tempdir().unwrap();
    let (s, _, _) = file_store(dir.path()).await;
    let bdir = dir.path().join("yedekler");
    std::fs::create_dir_all(&bdir).unwrap();
    for d in ["20200101", "20200102", "20200103"] {
        std::fs::write(bdir.join(format!("stok-{d}.db")), "x").unwrap();
    }
    std::fs::write(bdir.join("benim-dosyam.db"), "x").unwrap();
    let first = s.auto_backup(&bdir, 2).await.unwrap();
    assert!(first.is_some());
    assert!(s.auto_backup(&bdir, 2).await.unwrap().is_none(), "once per day");
    let mut names: Vec<_> = std::fs::read_dir(&bdir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names.contains(&"benim-dosyam.db".to_string()));
    assert!(names.contains(&"stok-20200103.db".to_string()));
    assert!(!names.contains(&"stok-20200101.db".to_string()));
}
