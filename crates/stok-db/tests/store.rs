use stok_db::Store;

async fn seed_one_variant_raw(s: &Store) {
    s.seed_ctx_for_test().await;
    let t = s.default_tenant_for_test().await;
    s.raw_exec_for_test(&format!(
        "INSERT INTO products (id, tenant_id, name, search_key, created_at, updated_at) VALUES ('p1', '{t}', 'Kupa', 'kupa', 0, 0)")).await.unwrap();
    s.raw_exec_for_test(&format!(
        "INSERT INTO variants (id, tenant_id, product_id, name, search_key, stock_qty, created_at, updated_at) VALUES ('v1', '{t}', 'p1', 'Standart', 'standart', 5, 0, 0)")).await.unwrap();
}

async fn seed_one_movement_raw(s: &Store) {
    seed_one_variant_raw(s).await;
    let t = s.default_tenant_for_test().await;
    s.raw_exec_for_test(&format!(
        "INSERT INTO stock_movements (id, tenant_id, variant_id, product_id, product_name, variant_name, kind, delta, quantity, source, idempotency_key, created_at, updated_at, created_by)
         VALUES ('m1', '{t}', 'v1', 'p1', 'Kupa', 'Standart', 'in', 5, 5, 'local', 'k1', 0, 0, 'u')")).await.unwrap();
    s.raw_exec_for_test(&format!(
        "INSERT INTO audit_log (id, tenant_id, entity, entity_id, action, created_at, updated_at, user_id) VALUES ('a1', '{t}', 'product', 'p1', 'create', 0, 0, 'u')")).await.unwrap();
}

#[tokio::test] async fn opens_and_migrates_file_db() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("stok.db");
    Store::open(&p).await.unwrap();
    let s = Store::open(&p).await.unwrap();
    assert_eq!(s.pragma_i64("application_id").await.unwrap(), 0x53544B31);
    assert_eq!(s.pragma_str("journal_mode").await.unwrap(), "wal");
}
#[tokio::test] async fn negative_stock_blocked_at_db_level() {
    let s = Store::open_in_memory().await.unwrap();
    seed_one_variant_raw(&s).await;
    assert!(s.raw_exec_for_test("UPDATE variants SET stock_qty = -1").await.is_err());
}
#[tokio::test] async fn ledger_and_audit_are_append_only() {
    let s = Store::open_in_memory().await.unwrap();
    seed_one_movement_raw(&s).await;
    assert!(s.raw_exec_for_test("DELETE FROM stock_movements").await.is_err());
    assert!(s.raw_exec_for_test("DELETE FROM audit_log").await.is_err());
    assert!(s.raw_exec_for_test("UPDATE stock_movements SET quantity = 99").await.is_err());
    assert!(s.raw_exec_for_test("UPDATE audit_log SET entity = 'x'").await.is_err());
    // the only allowed update: reversed_by NULL -> value, once
    s.raw_exec_for_test("UPDATE stock_movements SET reversed_by = 'm2'").await.unwrap();
    assert!(s.raw_exec_for_test("UPDATE stock_movements SET reversed_by = 'm3'").await.is_err());
}
#[tokio::test] async fn purge_guard_allows_delete_inside_tx() {
    let s = Store::open_in_memory().await.unwrap();
    seed_one_movement_raw(&s).await;
    s.delete_all_movements_with_guard_for_test().await.unwrap();
    assert_eq!(s.count_for_test("stock_movements").await, 0);
    assert_eq!(s.count_for_test("purge_guard").await, 0);
}
