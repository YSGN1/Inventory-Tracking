mod common;
use common::*;
use stok_core::catalog::*;
use stok_core::movement::*;
use stok_core::*;

#[tokio::test] async fn overdraw_rejected_without_partial_write() {
    let (s, ctx, v) = with_variant().await;
    s.record_movement(&ctx, mv_in(v, 3, "k1")).await.unwrap();
    assert!(matches!(s.record_movement(&ctx, mv_out(v, 5, "k2")).await, Err(DomainError::InsufficientStock{available:3, requested:5})));
    assert_eq!(stock(&s, &ctx, v).await, 3);
    assert_eq!(s.list_movements(&ctx, by_variant(v)).await.unwrap().len(), 1);
}
#[tokio::test] async fn same_idempotency_key_applied_once() {
    let (s, ctx, v) = with_variant().await;
    let a = s.record_movement(&ctx, mv_in(v, 10, "dup")).await.unwrap();
    let b = s.record_movement(&ctx, mv_in(v, 10, "dup")).await.unwrap();
    assert_eq!(a.id, b.id); assert_eq!(stock(&s, &ctx, v).await, 10);
}
#[tokio::test] async fn reversal_restores_and_keeps_history() {
    let (s, ctx, v) = with_variant().await;
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    let out = s.record_movement(&ctx, mv_out(v, 4, "b")).await.unwrap();
    assert_eq!(stock(&s, &ctx, v).await, 6);
    let r = s.reverse_movement(&ctx, out.id, "Yanlış giriş".into()).await.unwrap();
    assert_eq!(r.kind, MovementKind::Reversal);
    assert_eq!(r.reverses, Some(out.id));
    assert_eq!(r.delta, 4);
    assert_eq!(stock(&s, &ctx, v).await, 10);
    let list = s.list_movements(&ctx, by_variant(v)).await.unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list.iter().find(|m| m.movement.id == out.id).unwrap().movement.reversed_by, Some(r.id));
    assert!(matches!(s.reverse_movement(&ctx, out.id, "tekrar".into()).await, Err(DomainError::Validation{field:"movement",..})));
    assert!(matches!(s.reverse_movement(&ctx, r.id, "ters".into()).await, Err(DomainError::Validation{field:"movement",..})));
    let c = s.record_movement(&ctx, mv_in(v, 1, "c")).await.unwrap();
    assert!(matches!(s.reverse_movement(&ctx, c.id, "  ".into()).await, Err(DomainError::Validation{field:"reason",..})));
}
#[tokio::test] async fn reversal_that_would_go_negative_fails() {
    let (s, ctx, v) = with_variant().await;
    let a = s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    s.record_movement(&ctx, mv_out(v, 8, "b")).await.unwrap();
    assert!(matches!(s.reverse_movement(&ctx, a.id, "x".into()).await, Err(DomainError::InsufficientStock{..})));
    assert_eq!(stock(&s, &ctx, v).await, 2);
}
#[tokio::test] async fn correct_movement_replaces_effect() {
    let (s, ctx, v) = with_variant().await;
    let a = s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    let (rev, new) = s.correct_movement(&ctx, a.id, mv_in(v, 7, "a2")).await.unwrap();
    assert_eq!(rev.reverses, Some(a.id));
    assert_eq!(new.quantity, 7);
    assert_eq!(stock(&s, &ctx, v).await, 7);
    assert_eq!(s.list_movements(&ctx, by_variant(v)).await.unwrap().len(), 3);
}
#[tokio::test] async fn ledger_matches_cache() {
    let (s, ctx, v) = with_variant().await;
    let mut seed: u64 = 42;
    let mut next = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 33) as i64 };
    for i in 0..50 {
        let cur = stock(&s, &ctx, v).await;
        let m = match next() % 3 {
            0 => mv_in(v, 1 + next() % 20, &format!("k{i}")),
            1 if cur > 0 => mv_out(v, 1 + next() % cur, &format!("k{i}")),
            _ => mv_adjust(v, next() % 30, &format!("k{i}")),
        };
        s.record_movement(&ctx, m).await.unwrap();
    }
    let sum: i64 = s.list_movements(&ctx, by_variant(v)).await.unwrap().iter().map(|m| m.movement.delta).sum();
    assert_eq!(sum, stock(&s, &ctx, v).await);
}
#[tokio::test] async fn trashed_variant_rejects_movements_and_hides_history() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    s.record_movement(&ctx, mv_in(v, 2, "a")).await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    assert!(matches!(s.record_movement(&ctx, mv_in(v, 1, "b")).await, Err(DomainError::NotFound{entity:"variant"})));
    let mut q = by_variant(v);
    q.include_deleted = false;
    assert!(s.list_movements(&ctx, q.clone()).await.unwrap().is_empty());
    q.include_deleted = true;
    let rows = s.list_movements(&ctx, q).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].entity_deleted);
    assert!(s.stock_summary(&ctx, StockSummaryQuery::default()).await.unwrap().is_empty());
}
#[tokio::test] async fn summary_splits_in_out_adjust() {
    let (s, ctx, v) = with_variant().await;
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    s.record_movement(&ctx, mv_out(v, 3, "b")).await.unwrap();
    s.record_movement(&ctx, mv_adjust(v, 5, "c")).await.unwrap();
    let rows = s.stock_summary(&ctx, StockSummaryQuery::default()).await.unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!((r.total_in, r.total_out, r.total_adjust, r.current), (10, 3, -2, 5));
    assert_eq!((r.product_name.as_str(), r.variant_name.as_str()), ("Kupa", "Standart"));
}
#[tokio::test] async fn summary_puts_reversal_in_original_column() {
    let (s, ctx, v) = with_variant().await;
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    let out = s.record_movement(&ctx, mv_out(v, 3, "b")).await.unwrap();
    s.reverse_movement(&ctx, out.id, "hata".into()).await.unwrap();
    let r = &s.stock_summary(&ctx, StockSummaryQuery::default()).await.unwrap()[0];
    assert_eq!((r.total_in, r.total_out, r.total_adjust, r.current), (10, 0, 0, 10));
}
#[tokio::test] async fn movement_keeps_name_snapshot() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    s.record_movement(&ctx, mv_in(v, 2, "a")).await.unwrap();
    s.update_product(&ctx, p.product.id, patch_name("Bardak"), p.product.row_version).await.unwrap();
    let rows = s.list_movements(&ctx, by_variant(v)).await.unwrap();
    assert_eq!(rows[0].movement.product_name, "Kupa");
    assert_eq!(rows[0].user_name.as_deref(), Some("Test Yönetici"));
    let _ = ProductQuery::default();
}
#[tokio::test] async fn pagination_with_before() {
    let (s, ctx, v) = with_variant().await;
    for i in 0..5 { s.record_movement(&ctx, mv_in(v, 1, &format!("k{i}"))).await.unwrap(); }
    let mut q = by_variant(v); q.limit = 2;
    let page1 = s.list_movements(&ctx, q.clone()).await.unwrap();
    q.before = Some(page1[1].movement.id);
    let page2 = s.list_movements(&ctx, q).await.unwrap();
    assert_eq!(page1.len(), 2); assert_eq!(page2.len(), 2);
    assert!(page2[0].movement.id < page1[1].movement.id);
}
