mod common;
use common::*;
use stok_core::catalog::*;
use stok_core::*;
use stok_db::AuditAction;

#[tokio::test] async fn turkish_search_matches() {
    let (s, ctx) = seeded().await;
    s.create_product(&ctx, np("İpek Eşarp")).await.unwrap();
    s.create_product(&ctx, np("Ispanak Kutusu")).await.unwrap();
    assert_eq!(s.list_products(&ctx, q("ipek")).await.unwrap().len(), 1);
    assert!(s.list_products(&ctx, q("ispanak")).await.unwrap().is_empty());
    assert_eq!(s.list_products(&ctx, q("ISPANAK")).await.unwrap().len(), 1);
}
#[tokio::test] async fn stale_update_conflicts() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap().product;
    s.update_product(&ctx, p.id, patch_name("Kupa 1"), p.row_version).await.unwrap();
    assert!(matches!(s.update_product(&ctx, p.id, patch_name("Kupa 2"), p.row_version).await, Err(DomainError::Conflict)));
}
#[tokio::test] async fn trash_requires_confirm_word() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap().product;
    assert!(matches!(s.trash_product(&ctx, p.id, p.row_version, "sil").await, Err(DomainError::Validation{field:"confirm",..})));
    s.trash_product(&ctx, p.id, p.row_version, "SİL").await.unwrap();
    assert!(s.list_products(&ctx, q("kupa")).await.unwrap().is_empty());
}
#[tokio::test] async fn default_variant_created() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    assert_eq!(p.variants.len(), 1);
    assert_eq!(p.variants[0].name, "Standart");
    assert_eq!(p.total_stock, 0);
}
#[tokio::test] async fn explicit_variants_replace_default() {
    let (s, ctx) = seeded().await;
    let mut n = np("Tişört");
    n.variants = vec![
        VariantDraft { name: "S".into(), sku: None, min_stock: 2 },
        VariantDraft { name: "M".into(), sku: None, min_stock: 0 },
    ];
    let p = s.create_product(&ctx, n).await.unwrap();
    assert_eq!(p.variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["S", "M"]);
}
#[tokio::test] async fn duplicate_sku_rejected() {
    let (s, ctx) = seeded().await;
    let mut a = np("Kupa"); a.sku = Some("KP-1".into());
    s.create_product(&ctx, a.clone()).await.unwrap();
    a.name = "Kupa 2".into(); a.sku = Some("kp-1".into());
    assert!(matches!(s.create_product(&ctx, a).await, Err(DomainError::Validation{field:"sku",..})));
    // variant SKU clashing with product SKU is rejected too
    let p = s.create_product(&ctx, np("Bardak")).await.unwrap().product;
    let r = s.create_variant(&ctx, NewVariant { product_id: p.id, name: "Büyük".into(), sku: Some("KP-1".into()), min_stock: 0 }).await;
    assert!(matches!(r, Err(DomainError::Validation{field:"sku",..})));
}
#[tokio::test] async fn update_writes_audit_before_after() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap().product;
    s.update_product(&ctx, p.id, patch_name("Büyük Kupa"), p.row_version).await.unwrap();
    let h = s.audit_history(&ctx, "product", &p.id.to_string()).await.unwrap();
    assert_eq!(h.iter().map(|e| e.action).collect::<Vec<_>>(), [AuditAction::Update, AuditAction::Create]);
    assert_eq!(h[0].before.as_ref().unwrap()["name"], "Kupa");
    assert_eq!(h[0].after.as_ref().unwrap()["name"], "Büyük Kupa");
}
#[tokio::test] async fn last_variant_cannot_be_trashed_alone() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = &p.variants[0];
    assert!(matches!(s.trash_variant(&ctx, v.id, v.row_version, "SİL").await, Err(DomainError::Validation{field:"variant",..})));
    let v2 = s.create_variant(&ctx, NewVariant { product_id: p.product.id, name: "Büyük".into(), sku: None, min_stock: 0 }).await.unwrap();
    s.trash_variant(&ctx, v2.id, v2.row_version, "SİL").await.unwrap();
    assert_eq!(s.list_products(&ctx, q("kupa")).await.unwrap()[0].variants.len(), 1);
}
#[tokio::test] async fn variant_validation_and_locking() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = &p.variants[0];
    let neg = VariantPatch { min_stock: Some(-1), ..Default::default() };
    assert!(matches!(s.update_variant(&ctx, v.id, neg, v.row_version).await, Err(DomainError::Validation{field:"min_stock",..})));
    let ok = VariantPatch { min_stock: Some(4), ..Default::default() };
    let v1 = s.update_variant(&ctx, v.id, ok.clone(), v.row_version).await.unwrap();
    assert_eq!(v1.min_stock, 4);
    assert!(matches!(s.update_variant(&ctx, v.id, ok, v.row_version).await, Err(DomainError::Conflict)));
    assert!(matches!(s.create_product(&ctx, np("  ")).await, Err(DomainError::Validation{field:"name",..})));
}
