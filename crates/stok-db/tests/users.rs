use stok_core::user::*;
use stok_core::*;
use stok_db::Store;

async fn fresh_admin() -> (Store, Ctx, User) {
    let s = Store::open_in_memory().await.unwrap();
    let (ctx, u) = s.create_first_admin("yonetici", "Yönetici", "gizli123").await.unwrap();
    (s, ctx, u)
}

fn nu(username: &str, role: Role) -> NewUser {
    NewUser { username: username.into(), display_name: username.into(), password: "parola12".into(), role }
}

#[test] fn operator_permissions() {
    assert!(!can(Role::Operator, Permission::ViewDeleted));
    assert!(!can(Role::Operator, Permission::ManageTrash));
    assert!(!can(Role::Operator, Permission::ManageCatalog));
    assert!(can(Role::Operator, Permission::RecordMovement));
    assert!(can(Role::Operator, Permission::ViewAll));
    assert!(can(Role::Admin, Permission::ManageUsers));
}
#[tokio::test] async fn first_admin_only_once() {
    let s = Store::open_in_memory().await.unwrap();
    assert!(!s.is_initialized().await.unwrap());
    let (_, u) = s.create_first_admin("yonetici", "Yönetici", "gizli123").await.unwrap();
    assert_eq!(u.role, Role::Admin);
    assert!(s.is_initialized().await.unwrap());
    assert!(matches!(s.create_first_admin("baska", "Başka", "gizli123").await, Err(DomainError::Forbidden)));
}
#[tokio::test] async fn password_hashed_not_plain() {
    let (s, _, _) = fresh_admin().await;
    let h = s.password_hash_for_test("yonetici").await;
    assert!(h.starts_with("$argon2id$"), "{h}");
    assert!(!h.contains("gizli123"));
}
#[tokio::test] async fn wrong_password_and_inactive_same_error() {
    let (s, ctx, _) = fresh_admin().await;
    let op = s.create_user(&ctx, nu("ali", Role::Operator)).await.unwrap();
    assert!(s.authenticate("Ali", "parola12").await.is_ok(), "username is case-insensitive");
    assert!(matches!(s.authenticate("ali", "yanlis").await, Err(DomainError::InvalidCredentials)));
    assert!(matches!(s.authenticate("yok", "parola12").await, Err(DomainError::InvalidCredentials)));
    s.set_active(&ctx, op.id, false, op.row_version).await.unwrap();
    assert!(matches!(s.authenticate("ali", "parola12").await, Err(DomainError::InvalidCredentials)));
}
#[tokio::test] async fn cannot_demote_last_admin() {
    let (s, ctx, admin) = fresh_admin().await;
    assert!(matches!(s.set_role(&ctx, admin.id, Role::Operator, admin.row_version).await, Err(DomainError::Validation{field:"role",..})));
    assert!(matches!(s.set_active(&ctx, admin.id, false, admin.row_version).await, Err(DomainError::Validation{field:"role",..})));
    let a2 = s.create_user(&ctx, nu("ayse", Role::Admin)).await.unwrap();
    s.set_role(&ctx, admin.id, Role::Operator, admin.row_version).await.unwrap();
    assert!(matches!(s.set_active(&ctx, a2.id, false, a2.row_version).await, Err(DomainError::Validation{field:"role",..})));
}
#[tokio::test] async fn duplicate_username_and_short_password_rejected() {
    let (s, ctx, _) = fresh_admin().await;
    assert!(matches!(s.create_user(&ctx, nu("Yonetici", Role::Operator)).await, Err(DomainError::Validation{field:"username",..})));
    let mut weak = nu("veli", Role::Operator); weak.password = "123".into();
    assert!(matches!(s.create_user(&ctx, weak).await, Err(DomainError::Validation{field:"password",..})));
}
#[tokio::test] async fn set_password_changes_login() {
    let (s, ctx, admin) = fresh_admin().await;
    let u = s.set_password(&ctx, admin.id, "yeniparola", admin.row_version).await.unwrap();
    assert_eq!(u.row_version, admin.row_version + 1);
    assert!(s.authenticate("yonetici", "gizli123").await.is_err());
    assert!(s.authenticate("yonetici", "yeniparola").await.is_ok());
    assert_eq!(s.list_users(&ctx).await.unwrap().len(), 1);
}
#[tokio::test] async fn retention_setting_validated() {
    let (s, ctx, _) = fresh_admin().await;
    assert!(matches!(s.set_setting(&ctx, "trash_retention_days", "0").await, Err(DomainError::Validation{..})));
    assert!(matches!(s.set_setting(&ctx, "trash_retention_days", "366").await, Err(DomainError::Validation{..})));
    assert!(matches!(s.set_setting(&ctx, "trash_retention_days", "abc").await, Err(DomainError::Validation{..})));
    s.set_setting(&ctx, "trash_retention_days", "7").await.unwrap();
    assert_eq!(s.get_setting(&ctx, "trash_retention_days").await.unwrap().as_deref(), Some("7"));
    assert!(s.set_setting(&ctx, "single_user_mode", "2").await.is_err());
    s.set_setting(&ctx, "single_user_mode", "1").await.unwrap();
    s.set_setting(&ctx, "single_user_mode", "0").await.unwrap();
    assert_eq!(s.get_setting(&ctx, "single_user_mode").await.unwrap().as_deref(), Some("0"));
    assert!(s.set_setting(&ctx, "bilinmeyen", "1").await.is_err());
}
