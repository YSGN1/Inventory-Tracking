mod common;
use common::*;
use stok_core::firm::*;
use stok_core::*;

fn nf(name: &str) -> NewFirm {
    NewFirm { name: name.into(), contact: None, phone: None, note: None }
}

#[tokio::test] async fn duplicate_firm_name_case_insensitive() {
    let (s, ctx) = seeded().await;
    s.create_firm(&ctx, nf("ABC Matbaa")).await.unwrap();
    assert!(matches!(s.create_firm(&ctx, nf("abc matbaa")).await, Err(DomainError::Validation{field:"name",..})));
    assert!(matches!(s.create_firm(&ctx, nf("İz Ajans")).await, Ok(_)));
    assert!(matches!(s.create_firm(&ctx, nf("iz ajans")).await, Err(DomainError::Validation{field:"name",..})));
}
#[tokio::test] async fn stale_firm_update_conflicts() {
    let (s, ctx) = seeded().await;
    let f = s.create_firm(&ctx, nf("ABC")).await.unwrap();
    let patch = FirmPatch { phone: Some("0555".into()), ..Default::default() };
    let f2 = s.update_firm(&ctx, f.id, patch.clone(), f.row_version).await.unwrap();
    assert_eq!(f2.phone.as_deref(), Some("0555"));
    assert!(matches!(s.update_firm(&ctx, f.id, patch, f.row_version).await, Err(DomainError::Conflict)));
}
#[tokio::test] async fn out_to_trashed_firm_rejected() {
    let (s, ctx, v) = with_variant().await;
    let f = s.create_firm(&ctx, nf("ABC")).await.unwrap();
    s.record_movement(&ctx, mv_in(v, 5, "a")).await.unwrap();
    let m = s.record_movement(&ctx, mv_out_firm(v, 1, f.id, true, "b")).await.unwrap();
    assert_eq!(m.firm_name.as_deref(), Some("ABC"));
    s.trash_firm(&ctx, f.id, f.row_version, "SİL").await.unwrap();
    assert!(matches!(s.record_movement(&ctx, mv_out_firm(v, 1, f.id, true, "c")).await, Err(DomainError::NotFound{entity:"firm"})));
    assert!(s.list_firms(&ctx, None).await.unwrap().is_empty());
    // trashed firm name can be reused
    s.create_firm(&ctx, nf("ABC")).await.unwrap();
}
#[tokio::test] async fn trash_firm_requires_confirm() {
    let (s, ctx) = seeded().await;
    let f = s.create_firm(&ctx, nf("ABC")).await.unwrap();
    assert!(matches!(s.trash_firm(&ctx, f.id, f.row_version, "SIL").await, Err(DomainError::Validation{field:"confirm",..})));
    assert_eq!(s.list_firms(&ctx, Some("abc".into())).await.unwrap().len(), 1);
}
