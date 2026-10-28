#![allow(dead_code)]
use stok_core::catalog::*;
use stok_core::*;
use stok_db::*;

pub async fn seeded() -> (Store, Ctx) {
    let s = Store::open_in_memory().await.unwrap();
    let ctx = s.seed_ctx_for_test().await;
    (s, ctx)
}

pub fn np(name: &str) -> NewProduct {
    NewProduct { name: name.into(), sku: None, description: None, image_path: None, variants: vec![] }
}

pub fn q(search: &str) -> ProductQuery {
    ProductQuery { search: Some(search.into()) }
}

pub fn patch_name(name: &str) -> ProductPatch {
    ProductPatch { name: Some(name.into()), ..Default::default() }
}

/// Tek varyantlı "Kupa" ürünü.
pub async fn with_variant() -> (Store, Ctx, VariantId) {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    let v = p.variants[0].id;
    (s, ctx, v)
}

pub async fn stock(s: &Store, ctx: &Ctx, v: VariantId) -> i64 {
    s.get_variant(ctx, v).await.unwrap().stock_qty
}
