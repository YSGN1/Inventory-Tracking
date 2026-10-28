mod common;
use common::*;
use stok_core::catalog::*;
use stok_core::firm::NewFirm;
use stok_core::report::*;

#[tokio::test] async fn critical_includes_equal_threshold() {
    let (s, ctx) = seeded().await;
    let mut n = np("Kupa");
    n.variants = vec![
        VariantDraft { name: "Eşit".into(), sku: None, min_stock: 5 },
        VariantDraft { name: "Üstünde".into(), sku: None, min_stock: 5 },
        VariantDraft { name: "Eşiksiz".into(), sku: None, min_stock: 0 },
        VariantDraft { name: "Altında".into(), sku: None, min_stock: 5 },
    ];
    let p = s.create_product(&ctx, n).await.unwrap();
    let ids: Vec<_> = p.variants.iter().map(|v| v.id).collect();
    s.record_movement(&ctx, mv_in(ids[0], 5, "a")).await.unwrap();
    s.record_movement(&ctx, mv_in(ids[1], 6, "b")).await.unwrap();
    s.record_movement(&ctx, mv_in(ids[3], 1, "d")).await.unwrap();
    let mut t = np("Bardak");
    t.variants = vec![VariantDraft { name: "Çöpte".into(), sku: None, min_stock: 3 }];
    let trashed = s.create_product(&ctx, t).await.unwrap();
    s.trash_product(&ctx, trashed.product.id, trashed.product.row_version, "SİL").await.unwrap();

    let c = s.critical_stock(&ctx).await.unwrap();
    let names: Vec<_> = c.iter().map(|i| i.variant_name.as_str()).collect();
    assert_eq!(names, ["Altında", "Eşit"]);
}
#[tokio::test] async fn report_excludes_reversed_outs() {
    let (s, ctx, v) = with_variant().await;
    let f = s.create_firm(&ctx, NewFirm { name: "Öz Matbaa".into(), ..Default::default() }).await.unwrap();
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    let first = s.record_movement(&ctx, mv_out_firm(v, 4, f.id, true, "b")).await.unwrap();
    s.record_movement(&ctx, mv_out_firm(v, 2, f.id, false, "c")).await.unwrap();
    s.record_movement(&ctx, mv_out(v, 1, "no-firm")).await.unwrap();
    s.reverse_movement(&ctx, first.id, "hata".into()).await.unwrap();
    let (rows, totals) = s.firm_delivery_report(&ctx, DeliveryQuery { include_deleted: true, ..Default::default() }).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(totals.len(), 1);
    assert_eq!((totals[0].printed_qty, totals[0].unprinted_qty, totals[0].total_qty), (0, 2, 2));
    assert_eq!(totals[0].firm_name, "Öz Matbaa");
    let (rows, _) = s.firm_delivery_report(&ctx, DeliveryQuery { printed: Some(true), include_deleted: true, ..Default::default() }).await.unwrap();
    assert!(rows.is_empty());
}
#[tokio::test] async fn report_hides_deleted_unless_requested() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    let f = s.create_firm(&ctx, NewFirm { name: "ABC".into(), ..Default::default() }).await.unwrap();
    s.record_movement(&ctx, mv_in(v, 10, "a")).await.unwrap();
    s.record_movement(&ctx, mv_out_firm(v, 3, f.id, true, "b")).await.unwrap();
    s.trash_product(&ctx, p.product.id, p.product.row_version, "SİL").await.unwrap();
    let (rows, totals) = s.firm_delivery_report(&ctx, DeliveryQuery::default()).await.unwrap();
    assert!(rows.is_empty() && totals.is_empty());
    let (rows, _) = s.firm_delivery_report(&ctx, DeliveryQuery { include_deleted: true, ..Default::default() }).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].entity_deleted);
    assert!(!rows[0].firm_deleted);
}
