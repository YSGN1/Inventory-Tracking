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

use stok_core::movement::*;

fn mv(v: VariantId, kind: MovementKind, qty: i64, key: &str) -> NewMovement {
    NewMovement { variant_id: v, kind, quantity: qty, firm_id: None, printed: None, note: None,
        idempotency_key: key.into(), source: MovementSource::Local }
}
pub fn mv_in(v: VariantId, qty: i64, key: &str) -> NewMovement { mv(v, MovementKind::In, qty, key) }
pub fn mv_out(v: VariantId, qty: i64, key: &str) -> NewMovement { mv(v, MovementKind::Out, qty, key) }
pub fn mv_adjust(v: VariantId, qty: i64, key: &str) -> NewMovement { mv(v, MovementKind::Adjust, qty, key) }
pub fn mv_out_firm(v: VariantId, qty: i64, firm: FirmId, printed: bool, key: &str) -> NewMovement {
    NewMovement { firm_id: Some(firm), printed: Some(printed), ..mv(v, MovementKind::Out, qty, key) }
}

pub fn by_variant(v: VariantId) -> MovementQuery {
    MovementQuery { variant_id: Some(v), include_deleted: true, limit: 1000, ..Default::default() }
}

pub fn all_movements(include_deleted: bool) -> MovementQuery {
    MovementQuery { include_deleted, limit: 1000, ..Default::default() }
}
