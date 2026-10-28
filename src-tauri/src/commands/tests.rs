use stok_core::catalog::*;
use stok_core::movement::*;
use stok_core::trash::TrashKind;
use stok_core::user::{NewUser, Role};
use stok_core::*;
use tauri::Manager;
use tauri::test::{MockRuntime, mock_app};

use super::{auth, catalog, movements, settings, trash};
use crate::state::{AppPaths, AppState, startup};

type App = tauri::App<MockRuntime>;

async fn app_at(dir: &std::path::Path) -> App {
    let state = startup(AppPaths::under(dir)).await.unwrap();
    let app = mock_app();
    app.manage(state);
    app
}

async fn close(app: &App) {
    app.state::<AppState>().store().await.close().await;
}

async fn admin_app(dir: &std::path::Path) -> App {
    let app = app_at(dir).await;
    auth::setup_first_admin(app.state(), "yonetici".into(), "Yönetici".into(), "gizli123".into()).await.unwrap();
    app
}

async fn login_as_operator(app: &App) {
    auth::create_user(app.state(), NewUser { username: "ali".into(), display_name: "Ali".into(), password: "parola12".into(), role: Role::Operator })
        .await
        .unwrap();
    auth::logout(app.state()).await.unwrap();
    auth::login(app.state(), "ali".into(), "parola12".into()).await.unwrap();
}

fn np(name: &str) -> NewProduct {
    NewProduct { name: name.into(), ..Default::default() }
}

fn mv_in(v: VariantId, q: i64, key: &str) -> NewMovement {
    NewMovement { variant_id: v, kind: MovementKind::In, quantity: q, firm_id: None, printed: None, note: None, idempotency_key: key.into(), source: MovementSource::Local }
}

fn all() -> MovementQuery {
    MovementQuery { limit: 100, ..Default::default() }
}

#[tokio::test]
async fn commands_require_session() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_at(dir.path()).await;
    let e = catalog::list_products(app.state(), ProductQuery::default()).await.unwrap_err();
    assert_eq!(e.code, "unauthenticated");
    let st = auth::app_status(app.state()).await.unwrap();
    assert!(!st.initialized && st.user.is_none());
    close(&app).await;
}

#[tokio::test]
async fn operator_cannot_trash_or_purge() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    let p = catalog::create_product(app.state(), np("Kupa")).await.unwrap();
    login_as_operator(&app).await;
    let e = catalog::trash_product(app.state(), p.product.id, p.product.row_version, "SİL".into()).await.unwrap_err();
    assert_eq!(e.code, "forbidden");
    let e = trash::purge_item(app.state(), TrashKind::Product, p.product.id.to_string(), "SİL".into(), true).await.unwrap_err();
    assert_eq!(e.code, "forbidden");
    let e = catalog::create_product(app.state(), np("Bardak")).await.unwrap_err();
    assert_eq!(e.code, "forbidden");
    // operator can record movements
    movements::record_movement(app.state(), mv_in(p.variants[0].id, 3, "a")).await.unwrap();
    close(&app).await;
}

#[tokio::test]
async fn operator_never_sees_deleted_rows() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    let p = catalog::create_product(app.state(), np("Kupa")).await.unwrap();
    movements::record_movement(app.state(), mv_in(p.variants[0].id, 3, "a")).await.unwrap();
    catalog::trash_product(app.state(), p.product.id, p.product.row_version, "SİL".into()).await.unwrap();
    let rows = movements::list_movements(app.state(), all()).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].movement.product_name, "Silinmiş ürün: Kupa");
    login_as_operator(&app).await;
    assert!(movements::list_movements(app.state(), all()).await.unwrap().is_empty());
    close(&app).await;
}

#[tokio::test]
async fn image_rejects_non_image_and_large_files() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    let p = catalog::create_product(app.state(), np("Kupa")).await.unwrap();
    let src = tempfile::tempdir().unwrap();
    let fake = src.path().join("sahte.png");
    std::fs::write(&fake, b"merhaba").unwrap();
    let e = catalog::set_product_image(app.state(), p.product.id, fake.to_string_lossy().into()).await.unwrap_err();
    assert_eq!(e.code, "validation");
    let txt = src.path().join("not.txt");
    std::fs::write(&txt, b"\x89PNG\r\n\x1a\n").unwrap();
    assert_eq!(catalog::set_product_image(app.state(), p.product.id, txt.to_string_lossy().into()).await.unwrap_err().code, "validation");
    let big = src.path().join("buyuk.png");
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.resize(5 * 1024 * 1024 + 1, 0);
    std::fs::write(&big, &bytes).unwrap();
    assert_eq!(catalog::set_product_image(app.state(), p.product.id, big.to_string_lossy().into()).await.unwrap_err().code, "validation");

    let ok = src.path().join("kupa.PNG");
    std::fs::write(&ok, b"\x89PNG\r\n\x1a\nveri").unwrap();
    let prod = catalog::set_product_image(app.state(), p.product.id, ok.to_string_lossy().into()).await.unwrap();
    assert_eq!(prod.image_path, Some(format!("{}.png", p.product.id)));
    let data = catalog::get_product_image(app.state(), p.product.id).await.unwrap().unwrap();
    assert!(data.starts_with("data:image/png;base64,"));
    // client cannot point image_path at an arbitrary file
    let patch = ProductPatch { image_path: Some("../../secret.txt".into()), ..Default::default() };
    let after = catalog::update_product(app.state(), p.product.id, patch, prod.row_version).await.unwrap();
    assert_eq!(after.image_path, prod.image_path);
    close(&app).await;
}

#[tokio::test]
async fn single_user_mode_auto_logs_in() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    settings::set_single_user_mode(app.state(), true).await.unwrap();
    close(&app).await;
    drop(app);
    let app = app_at(dir.path()).await;
    let st = auth::app_status(app.state()).await.unwrap();
    assert!(st.single_user_mode);
    assert_eq!(st.user.unwrap().username, "yonetici");
    close(&app).await;
}

#[tokio::test]
async fn startup_purges_expired() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    let p = catalog::create_product(app.state(), np("Kupa")).await.unwrap();
    catalog::trash_product(app.state(), p.product.id, p.product.row_version, "SİL".into()).await.unwrap();
    let old = now_ms() - 40 * 86_400_000;
    let s = app.state::<AppState>().store().await;
    s.raw_exec_for_test(&format!("UPDATE products SET deleted_at = {old}")).await.unwrap();
    assert_eq!(trash::list_trash(app.state()).await.unwrap().len(), 1);
    close(&app).await;
    drop(app);
    let app = app_at(dir.path()).await;
    auth::login(app.state(), "yonetici".into(), "gizli123".into()).await.unwrap();
    assert!(trash::list_trash(app.state()).await.unwrap().is_empty());
    let backups = std::fs::read_dir(dir.path().join("yedekler")).unwrap().count();
    assert_eq!(backups, 1, "one auto-backup per day");
    close(&app).await;
}

#[tokio::test]
async fn restore_backup_swaps_database() {
    let dir = tempfile::tempdir().unwrap();
    let app = admin_app(dir.path()).await;
    let bak = dir.path().join("elle.db");
    settings::backup_now(app.state(), bak.to_string_lossy().into()).await.unwrap();
    catalog::create_product(app.state(), np("Kupa")).await.unwrap();
    let st = settings::restore_backup(app.state(), bak.to_string_lossy().into()).await.unwrap();
    assert!(st.user.is_none(), "session cleared after restore");
    auth::login(app.state(), "yonetici".into(), "gizli123".into()).await.unwrap();
    assert!(catalog::list_products(app.state(), ProductQuery::default()).await.unwrap().is_empty());
    let e = settings::restore_backup(app.state(), dir.path().join("yok.db").to_string_lossy().into()).await.unwrap_err();
    assert_eq!(e.code, "validation");
    // still works after rejected restore
    assert!(catalog::list_products(app.state(), ProductQuery::default()).await.is_ok());
    close(&app).await;
}
