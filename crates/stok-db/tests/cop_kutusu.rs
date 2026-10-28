mod common;
use common::*;
use stok_core::catalog::*;
use stok_core::firm::NewFirm;
use stok_core::movement::*;
use stok_core::trash::*;
use stok_core::*;
use stok_db::AuditAction;

const DAY: i64 = 86_400_000;

#[tokio::test] async fn restore_brings_back_product_with_stock() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    s.record_movement(&ctx, mv_in(v, 7, "a")).await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let t = s.list_trash(&ctx).await.unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].kind, t[0].stock_qty, t[0].name.as_str()), (TrashKind::Product, 7, "Kupa"));
    assert_eq!(t[0].purge_at - t[0].deleted_at, 30 * DAY);
    s.restore(&ctx, TrashKind::Product, &t[0].id, t[0].row_version).await.unwrap();
    let sum = s.stock_summary(&ctx, StockSummaryQuery::default()).await.unwrap();
    assert_eq!(sum[0].current, 7);
    assert!(s.list_trash(&ctx).await.unwrap().is_empty());
    let h = s.audit_history(&ctx, "product", &p.product.id.to_string()).await.unwrap();
    assert_eq!(h[0].action, AuditAction::Restore);
}
#[tokio::test] async fn restore_keeps_separately_trashed_variant_in_trash() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v2 = s.create_variant(&ctx, NewVariant { product_id: p.product.id, name: "Büyük".into(), sku: None, min_stock: 0 }).await.unwrap();
    s.trash_variant(&ctx, v2.id, v2.row_version, "SİL").await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let t = s.list_trash(&ctx).await.unwrap();
    assert_eq!(t.len(), 1, "variant of trashed product not listed separately");
    s.restore(&ctx, TrashKind::Product, &t[0].id, t[0].row_version).await.unwrap();
    let t = s.list_trash(&ctx).await.unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].kind, t[0].parent_name.as_deref()), (TrashKind::Variant, Some("Kupa")));
    assert_eq!(s.get_product(&ctx, p.product.id).await.unwrap().variants.len(), 1);
}
#[tokio::test] async fn restore_fails_on_sku_clash() {
    let (s, ctx) = seeded().await;
    let mut a = np("Kupa"); a.sku = Some("KP-1".into());
    let p = s.create_product(&ctx, a.clone()).await.unwrap().product;
    s.trash_product(&ctx, p.id, p.row_version, "SİL").await.unwrap();
    a.name = "Yeni Kupa".into();
    s.create_product(&ctx, a).await.unwrap();
    let t = s.list_trash(&ctx).await.unwrap();
    let r = s.restore(&ctx, TrashKind::Product, &t[0].id, t[0].row_version).await;
    assert!(matches!(r, Err(DomainError::Validation{field:"sku", ref message}) if message == "Bu SKU başka bir üründe kullanılıyor"));
}
#[tokio::test] async fn restore_fails_on_firm_name_clash_and_stale_version() {
    let (s, ctx) = seeded().await;
    let f = s.create_firm(&ctx, NewFirm { name: "ABC".into(), ..Default::default() }).await.unwrap();
    s.trash_firm(&ctx, f.id, f.row_version, "SİL").await.unwrap();
    let t = s.list_trash(&ctx).await.unwrap();
    assert!(matches!(s.restore(&ctx, TrashKind::Firm, &t[0].id, t[0].row_version - 1).await, Err(DomainError::Conflict)));
    s.create_firm(&ctx, NewFirm { name: "abc".into(), ..Default::default() }).await.unwrap();
    assert!(matches!(s.restore(&ctx, TrashKind::Firm, &t[0].id, t[0].row_version).await, Err(DomainError::Validation{field:"name",..})));
}
#[tokio::test] async fn variant_restore_requires_parent() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v2 = s.create_variant(&ctx, NewVariant { product_id: p.product.id, name: "Büyük".into(), sku: None, min_stock: 0 }).await.unwrap();
    s.trash_variant(&ctx, v2.id, v2.row_version, "SİL").await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let r = s.restore(&ctx, TrashKind::Variant, &v2.id.to_string(), v2.row_version + 1).await;
    assert!(matches!(r, Err(DomainError::Validation{field:"product",..})));
}
#[tokio::test] async fn purge_requires_confirm_and_trashed_state() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap().product;
    let id = p.id.to_string();
    assert!(matches!(s.purge(&ctx, TrashKind::Product, &id, "sil", false).await, Err(DomainError::Validation{field:"confirm",..})));
    assert!(matches!(s.purge(&ctx, TrashKind::Product, &id, "SİL", false).await, Err(DomainError::Validation{field:"state",..})));
}
#[tokio::test] async fn purge_keeps_history_by_default() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    s.record_movement(&ctx, mv_in(v, 5, "a")).await.unwrap();
    s.record_movement(&ctx, mv_out(v, 2, "b")).await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let rep = s.purge(&ctx, TrashKind::Product, &p.product.id.to_string(), "SİL", false).await.unwrap();
    assert_eq!((rep.removed_entities, rep.removed_movements), (2, 0));
    let rows = s.list_movements(&ctx, all_movements(true)).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].movement.product_name, "Kupa");
    assert_eq!(rows[0].movement.variant_id, None);
    assert!(rows[0].entity_deleted);
    assert!(s.list_movements(&ctx, all_movements(false)).await.unwrap().is_empty());
    assert!(s.list_trash(&ctx).await.unwrap().is_empty());
    let h = s.audit_history(&ctx, "product", &p.product.id.to_string()).await.unwrap();
    assert_eq!(h[0].action, AuditAction::Purge);
    assert!(h.len() > 1, "history kept");
}
#[tokio::test] async fn purge_with_history_removes_movements_and_audit() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    s.record_movement(&ctx, mv_in(v, 5, "a")).await.unwrap();
    let out = s.record_movement(&ctx, mv_out(v, 2, "b")).await.unwrap();
    s.reverse_movement(&ctx, out.id, "hata".into()).await.unwrap();
    // another product's movement must survive
    let (other, ov) = { let o = s.create_product(&ctx, np("Bardak")).await.unwrap(); let id = o.variants[0].id; (o, id) };
    s.record_movement(&ctx, mv_in(ov, 1, "o")).await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let rep = s.purge(&ctx, TrashKind::Product, &p.product.id.to_string(), "SİL", true).await.unwrap();
    assert_eq!(rep.removed_movements, 3);
    let rows = s.list_movements(&ctx, all_movements(true)).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].movement.product_name, "Bardak");
    let h = s.audit_history(&ctx, "product", &p.product.id.to_string()).await.unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].action, AuditAction::Purge);
    assert!(s.audit_history(&ctx, "variant", &v.to_string()).await.unwrap().is_empty());
    assert_eq!(s.get_product(&ctx, other.product.id).await.unwrap().total_stock, 1);
}
#[tokio::test] async fn firm_purge_with_history_keeps_stock() {
    let (s, ctx, v) = with_variant().await;
    let f = s.create_firm(&ctx, NewFirm { name: "ABC".into(), ..Default::default() }).await.unwrap();
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    s.record_movement(&ctx, mv_out_firm(v, 3, f.id, true, "b")).await.unwrap();
    s.trash_firm(&ctx, f.id, f.row_version, "SİL").await.unwrap();
    let rep = s.purge(&ctx, TrashKind::Firm, &f.id.to_string(), "SİL", true).await.unwrap();
    assert!(rep.note.is_some());
    assert_eq!(rep.removed_movements, 0);
    assert_eq!(stock(&s, &ctx, v).await, 7);
    let rows = s.list_movements(&ctx, all_movements(true)).await.unwrap();
    assert_eq!(rows.len(), 2);
    let out = rows.iter().find(|r| r.movement.kind == MovementKind::Out).unwrap();
    assert_eq!(out.movement.firm_id, None);
}
#[tokio::test] async fn expired_items_purged_automatically() {
    let (s, ctx) = seeded().await;
    let a = s.create_product(&ctx, np("Eski")).await.unwrap().product;
    let b = s.create_product(&ctx, np("Yeni")).await.unwrap().product;
    s.trash_product(&ctx, a.id, a.row_version, "SİL").await.unwrap();
    s.trash_product(&ctx, b.id, b.row_version, "SİL").await.unwrap();
    let now = now_ms();
    s.raw_exec_for_test(&format!("UPDATE products SET deleted_at = {} WHERE id = '{}'", now - 31 * DAY, a.id)).await.unwrap();
    s.raw_exec_for_test(&format!("UPDATE variants SET deleted_at = {} WHERE product_id = '{}'", now - 31 * DAY, a.id)).await.unwrap();
    s.raw_exec_for_test(&format!("UPDATE products SET deleted_at = {} WHERE id = '{}'", now - 29 * DAY, b.id)).await.unwrap();
    assert_eq!(s.purge_expired(&ctx, now).await.unwrap(), 1);
    assert_eq!(s.purge_expired(&ctx, now).await.unwrap(), 0);
    let t = s.list_trash(&ctx).await.unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].name, "Yeni");
}
#[test] fn retention_bounds() {
    assert!(validate_retention_days(0).is_err());
    assert!(validate_retention_days(366).is_err());
    assert!(validate_retention_days(1).is_ok() && validate_retention_days(365).is_ok());
}
