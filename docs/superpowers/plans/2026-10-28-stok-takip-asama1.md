# Stok Takip — Aşama 1 (Çekirdek Masaüstü) Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows, macOS ve Linux'ta yerel çalışan, çok kullanıcılı, varyantlı, hareket defteri tabanlı, çöp kutulu bir stok takip masaüstü uygulaması.

**Architecture:** Cargo workspace: saf iş kuralları `stok-core`'da (IO yok), SQLite erişimi `stok-db`'de (`Store`), Tauri 2 kabuğu `src-tauri`'de (komutlar + oturum + yetki + rol bazlı görünürlük), arayüz `ui/`'de (SvelteKit SPA). Stok, hareket defterinden türetilir; hareket düzeltme/geri alma ters kayıtla yapılır, tüm değişiklikler `audit_log`'a yazılır. Silme iki aşamalı: çöp kutusu (`deleted_at`) → kalıcı silme (`purge`, isteğe bağlı geçmişiyle). Hareketler ad kopyası taşıdığı için kalıcı silinen ürünün geçmişi okunabilir kalır. Her satır `tenant_id`, UUID v7, `row_version` taşır → Aşama 2 (site) ve Aşama 3 (PostgreSQL, çok kiracılık) şema değişikliği gerektirmez.

**Tech Stack:** Rust (edition 2024, ≥1.85), Tauri 2.x (son kararlı), sqlx 0.8 (`sqlite`, `runtime-tokio`, `uuid`, `macros`, `migrate`), uuid 1 (`v7`, `serde`), argon2 0.5, thiserror 2, serde/serde_json, ts-rs 10, csv 1, chrono + chrono-tz, tokio. UI: SvelteKit 2 + Svelte 5 + TypeScript + Vite + Tailwind CSS 4 + `@sveltejs/adapter-static`, `@tauri-apps/api` 2, `@tauri-apps/plugin-dialog` 2, Vitest + @testing-library/svelte. CI: GitHub Actions + `tauri-apps/tauri-action`.

**Spec:** `docs/superpowers/specs/2026-10-28-stok-takip-tasarim.md`

## Global Constraints

- Hedef platformlar: Windows 10+, macOS 11+, Linux (Ubuntu 22.04+ / WebKitGTK 4.1). Platforma özel kod yok; tüm yollar Tauri `app_data_dir` üzerinden, yol birleştirme `PathBuf::join` ile.
- Tüm tablolarda: `id TEXT PRIMARY KEY` (UUID v7), `tenant_id TEXT NOT NULL`, `created_at INTEGER`, `updated_at INTEGER` (UTC epoch ms), `row_version INTEGER NOT NULL DEFAULT 1`, `deleted_at INTEGER NULL`. İstisna: `stock_movements` ve `audit_log` yalnız INSERT alır (tek istisna: `stock_movements.reversed_by` bir kez yazılır).
- Fiziksel `DELETE` yalnız `Store::purge_*` fonksiyonlarında ve otomatik çöp temizliğinde yapılır. `stock_movements` ve `audit_log` silmesi veritabanı tetikleyicisiyle engellenir; yalnız "geçmişiyle birlikte sil" işlemi aynı transaction içinde `purge_guard` satırı ekleyerek bu korumayı açar.
- Silme ve kalıcı silme komutları `confirm` parametresi alır; değer tam olarak `"SİL"` değilse `Validation{field:"confirm"}`. Kontrol arka uçta yapılır (UI'da da aynı kural).
- Silinmiş (çöpte veya kalıcı silinmiş) kayıtlara ait geçmiş satırlarını yalnız Admin görür, etiket `"Silinmiş ürün: <ad>"` / `"Silinmiş firma: <ad>"`. Operatörde bu satırlar dönmez. Filtre komut katmanında rolden belirlenir (`Store` sorguları `include_deleted: bool` alır).
- Miktarlar `i64` adet; stok hiçbir zaman negatif olamaz.
- SQL yalnızca SQLite ve PostgreSQL'de ortak çalışan söz dizimiyle (`ON CONFLICT`, `RETURNING` serbest; `AUTOINCREMENT`, `datetime()`, `GLOB` yasak). Tetikleyiciler migration'da ayrı blokta; Aşama 3'te PostgreSQL karşılıkları yazılır.
- SQLite: `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000`, `PRAGMA application_id = 0x53544B31`.
- Her yazma işlemi tek transaction: veri + `stock_qty` önbelleği + `audit_log`.
- İyimser kilitleme: güncelleme/silme/geri getirme `expected_version` alır; uyuşmazsa `DomainError::Conflict`.
- Kullanıcıya görünen tüm metinler Türkçe; arayüz metinleri `ui/src/lib/i18n/tr.ts`.
- Türkçe büyük/küçük harf duyarsız arama: `search_key` sütunu Rust'ta `normalize_tr()` ile üretilir.
- Ana stok ekranında yalnız: Ürün/Varyant, Giren, Çıkan, Düzeltme, Mevcut.
- Çöp kutusu saklama süresi `app_settings.trash_retention_days`, varsayılan `30`, izinli aralık 1–365.
- Parola: argon2id; düz parola saklanmaz/loglanmaz.
- Tauri capability yalnız `core:default`, `dialog:allow-open`, `dialog:allow-save`; uzak URL yüklenmez; dosya işlemleri Rust tarafında.

## Review Focus

1. **Mevcut stoktan fazla çıkış** → reddedilir, "Yetersiz stok: mevcut X, istenen Y"; kısmi yazım olmaz. (Task 2, Task 5)
2. **Aynı hareketin iki kez gönderilmesi (çift tıklama)** → aynı `idempotency_key` stoğu bir kez değiştirir. (Task 5)
3. **İki kullanıcının aynı kaydı aynı anda düzenlemesi** → ikincisi `Conflict`, "Kayıt başka biri tarafından değiştirildi, lütfen yenileyin". (Task 4)
4. **Çöpteki ürünün varyantına hareket girilmesi / çöpten geri getirilen ürünün stoğu** → çöpteki varyanta hareket `NotFound`; geri getirilince stok aynen döner; çöpteki ürün özet ve kritik listede görünmez. (Task 5, Task 8)
5. **Geri getirilen ürünle aynı SKU'lu yeni ürün eklenmişse** → geri getirme `Validation{field:"sku"}` ile reddedilir, "Bu SKU başka bir üründe kullanılıyor". (Task 8)

---

## Dosya Yapısı

```
stok-takip/
  Cargo.toml                       # workspace
  .github/workflows/release.yml    # Windows/macOS/Linux derleme
  crates/stok-core/src/
    lib.rs ids.rs error.rs text.rs ctx.rs
    catalog.rs firm.rs movement.rs user.rs report.rs trash.rs
  crates/stok-db/
    migrations/0001_init.sql
    src/lib.rs store.rs audit.rs catalog.rs firms.rs movements.rs
        reports.rs trash.rs users.rs backup.rs
    tests/ store.rs catalog.rs firms.rs movements.rs reports.rs trash.rs users.rs backup.rs
  src-tauri/
    tauri.conf.json capabilities/default.json
    src/main.rs state.rs error.rs visibility.rs
        commands/{auth,catalog,firms,movements,reports,trash,settings}.rs
  ui/
    src/lib/api.ts src/lib/bindings/ src/lib/i18n/tr.ts
    src/lib/components/ src/routes/
```

---

### Task 1: Workspace ve çekirdek tipler

**Files:**
- Create: `Cargo.toml`, `crates/stok-core/Cargo.toml`, `crates/stok-core/src/{lib,ids,error,text,ctx}.rs`
- Test: birim test modülleri `text.rs`, `ids.rs`, `error.rs`

**Interfaces:**
- Produces:
  - `id_type!` makrosuyla `TenantId, UserId, ProductId, VariantId, FirmId, MovementId` — `pub struct X(pub Uuid)`; `X::new()` (UUID v7), `Display`, `FromStr`, `Serialize/Deserialize`, `ts_rs::TS` (string).
  - `pub enum DomainError { NotFound{entity:&'static str}, Validation{field:&'static str, message:String}, InsufficientStock{available:i64, requested:i64}, Conflict, Forbidden, InvalidCredentials, Storage(String) }` + `fn code(&self) -> &'static str` + `fn user_message(&self) -> String`.
  - `pub fn normalize_tr(s: &str) -> String`
  - `pub const DELETE_CONFIRM_WORD: &str = "SİL";` + `pub fn check_delete_confirmation(input: &str) -> Result<(), DomainError>` (`trim()` sonrası birebir eşitlik; büyük/küçük harf duyarlı).
  - `pub struct Ctx { pub tenant_id: TenantId, pub user_id: UserId }`, `pub fn now_ms() -> i64`

- [ ] **Step 1: `git init`, workspace (members: `crates/*`, `src-tauri`), `stok-core` iskeleti, `.gitignore`.**
- [ ] **Step 2: Failing testler**

```rust
#[test] fn normalize_tr_handles_dotted_i() {
    assert_eq!(normalize_tr("İPEK Iğdır"), "ipek ığdır");
    assert_ne!(normalize_tr("ISPANAK"), normalize_tr("ispanak"));
    assert_eq!(normalize_tr("  Çanta  "), "çanta");
}
#[test] fn ids_are_time_ordered() {
    let a = ProductId::new(); let b = ProductId::new();
    assert!(a.0 < b.0);
    assert_eq!(a.to_string().parse::<ProductId>().unwrap(), a);
}
#[test] fn insufficient_stock_message_is_turkish() {
    let e = DomainError::InsufficientStock{available:3, requested:5};
    assert_eq!(e.user_message(), "Yetersiz stok: mevcut 3, istenen 5");
    assert_eq!(e.code(), "insufficient_stock");
}
#[test] fn delete_confirmation_requires_exact_word() {
    assert!(check_delete_confirmation("SİL").is_ok());
    assert!(check_delete_confirmation(" SİL ").is_ok());
    for bad in ["sil", "SIL", "Sil", "", "SİLL"] {
        assert!(matches!(check_delete_confirmation(bad), Err(DomainError::Validation{field:"confirm",..})));
    }
}
```

- [ ] **Step 3: `cargo test -p stok-core` → FAIL**
- [ ] **Step 4: Uygula.** `normalize_tr`: `İ→i`, `I→ı`, sonra `to_lowercase()`, `trim()`. Mesajlar: `Conflict` → "Kayıt başka biri tarafından değiştirildi, lütfen yenileyin"; `Forbidden` → "Bu işlem için yetkiniz yok"; `InvalidCredentials` → "Kullanıcı adı veya parola hatalı"; confirm → "Onaylamak için SİL yazın".
- [ ] **Step 5: PASS**
- [ ] **Step 6: Commit** `feat(core): ids, errors, Turkish normalization, delete confirmation`

---

### Task 2: Hareket kuralları (saf mantık)

**Files:**
- Create: `crates/stok-core/src/movement.rs` (+ `#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub enum MovementKind { In, Out, Adjust, Reversal }` (serde `snake_case`), `pub enum MovementSource { Local, Site }`
  - `pub struct NewMovement { variant_id: VariantId, kind: MovementKind, quantity: i64, firm_id: Option<FirmId>, printed: Option<bool>, note: Option<String>, idempotency_key: String, source: MovementSource }`
  - `pub struct Movement { id, variant_id: Option<VariantId>, product_id: Option<ProductId>, product_name: String, variant_name: String, firm_id: Option<FirmId>, firm_name: Option<String>, kind, delta: i64, quantity: i64, printed: Option<bool>, note, source, reverses: Option<MovementId>, reversed_by: Option<MovementId>, created_at: i64, created_by: UserId }` — `*_id` alanları kalıcı silmeden sonra `None` olabilir; `*_name` alanları kayıt anındaki ad kopyasıdır.
  - `pub fn validate(m: &NewMovement) -> Result<(), DomainError>`
  - `pub fn compute_delta(kind: MovementKind, quantity: i64, current: i64) -> Result<i64, DomainError>`
  - `pub fn apply_delta(current: i64, delta: i64) -> Result<i64, DomainError>`

Kurallar: `In` → +quantity; `Out` → −quantity; `Adjust` → `quantity` sayım sonucu, delta = quantity − current. `In/Out` için `quantity > 0`, `Adjust` için `≥ 0`. `printed` ve `firm_id` yalnız `Out`'ta. `Reversal` dışarıdan gönderilemez. `idempotency_key` boş olamaz. Not ≤ 500 karakter.

- [ ] **Step 1: Failing testler**

```rust
#[test] fn out_more_than_stock_fails() {
    let d = compute_delta(MovementKind::Out, 5, 3).unwrap();
    assert!(matches!(apply_delta(3, d), Err(DomainError::InsufficientStock{available:3, requested:5})));
}
#[test] fn adjust_sets_absolute_count() { assert_eq!(compute_delta(MovementKind::Adjust, 7, 10).unwrap(), -3); }
#[test] fn zero_in_rejected() {
    assert!(matches!(validate(&new_mv(MovementKind::In, 0)), Err(DomainError::Validation{field:"quantity",..})));
}
#[test] fn printed_only_on_out() {
    let mut m = new_mv(MovementKind::In, 2); m.printed = Some(true);
    assert!(matches!(validate(&m), Err(DomainError::Validation{field:"printed",..})));
}
#[test] fn reversal_cannot_be_submitted() { assert!(validate(&new_mv(MovementKind::Reversal, 1)).is_err()); }
#[test] fn out_to_exact_zero_ok() { assert_eq!(apply_delta(4, -4).unwrap(), 0); }
```

- [ ] **Step 2: `cargo test -p stok-core movement` → FAIL**
- [ ] **Step 3: Uygula** (`apply_delta` < 0 → `InsufficientStock{available: current, requested: -delta}`).
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(core): movement rules`

---

### Task 3: Veritabanı şeması ve `Store`

**Files:**
- Create: `crates/stok-db/Cargo.toml`, `crates/stok-db/migrations/0001_init.sql`, `crates/stok-db/src/{lib,store,audit}.rs`
- Test: `crates/stok-db/tests/store.rs`

**Interfaces:**
- Produces:
  - `pub struct Store { pool: SqlitePool }`; `Store::open(path: &Path) -> Result<Store, DomainError>`; `Store::open_in_memory()`.
  - `pub(crate) async fn write_audit(tx, ctx, entity: &str, entity_id: &str, action: AuditAction, before: Option<Value>, after: Option<Value>)`
  - `pub enum AuditAction { Create, Update, Delete, Restore, Purge }`
  - `Store::audit_history(&self, ctx, entity: &str, entity_id: &str) -> Result<Vec<AuditEntry>>` (yeniden eskiye)
  - `pub(crate) async fn with_purge_guard<T>(tx, f) -> Result<T>` — `purge_guard`'a satır ekler, `f`'yi çalıştırır, satırı siler (aynı transaction).

Tablolar: `tenants`, `users`, `products`, `variants` (`stock_qty INTEGER NOT NULL DEFAULT 0 CHECK (stock_qty >= 0)`, `min_stock INTEGER NOT NULL DEFAULT 0`), `firms`, `stock_movements` (`variant_id`, `product_id`, `firm_id` **FK yok, NULL olabilir**; `product_name`, `variant_name`, `firm_name` ad kopyaları; `UNIQUE(tenant_id, idempotency_key)`), `audit_log`, `app_settings (tenant_id, key, value)`, `external_refs` (Aşama 2 için boş), `purge_guard (id INTEGER)`. `search_key` sütunları + indeksler. Tetikleyiciler: `stock_movements` ve `audit_log` için `BEFORE DELETE ... WHEN NOT EXISTS (SELECT 1 FROM purge_guard)` → `RAISE(ABORT, 'append_only')`; `stock_movements` için `BEFORE UPDATE` → yalnız `reversed_by` NULL'dan değere geçişe izin. İlk açılışta varsayılan kiracı satırı.

- [ ] **Step 1: Failing testler**

```rust
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
}
#[tokio::test] async fn purge_guard_allows_delete_inside_tx() {
    let s = Store::open_in_memory().await.unwrap();
    seed_one_movement_raw(&s).await;
    s.delete_all_movements_with_guard_for_test().await.unwrap();
    assert_eq!(s.count_for_test("stock_movements").await, 0);
    assert_eq!(s.count_for_test("purge_guard").await, 0);
}
```
(Test yardımcıları `#[cfg(any(test, feature = "test-util"))]`.)

- [ ] **Step 2: FAIL** — `cargo test -p stok-db --test store`
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(db): schema, triggers, store, audit`

---

### Task 4: Ürün ve varyant yönetimi

**Files:**
- Create: `crates/stok-core/src/catalog.rs`, `crates/stok-db/src/catalog.rs`
- Test: `crates/stok-db/tests/catalog.rs`

**Interfaces:**
- Produces:
  - `NewProduct { name, sku: Option<String>, description: Option<String>, image_path: Option<String> }`, `ProductPatch` (aynı alanlar `Option`), `NewVariant { product_id, name, sku: Option<String>, min_stock: i64 }`, `VariantPatch { name, sku, min_stock }`.
  - `Product { id, name, sku, description, image_path, row_version, created_at, updated_at, deleted_at }`, `Variant { id, product_id, name, sku, min_stock, stock_qty, row_version, deleted_at }`, `ProductWithVariants { product, variants: Vec<Variant>, total_stock: i64 }`.
  - `Store::create_product(ctx, NewProduct) -> Result<Product>` — varyant verilmezse `"Standart"` varyantı oluşturur.
  - `Store::update_product(ctx, ProductId, ProductPatch, expected_version) -> Result<Product>`
  - `Store::trash_product(ctx, ProductId, expected_version, confirm: &str) -> Result<()>` — onay kontrolü; ürün + varyantlara aynı `deleted_at`; stok olsa da izinli; audit `Delete`.
  - `Store::list_products(ctx, ProductQuery { search: Option<String> }) -> Result<Vec<ProductWithVariants>>` — çöptekiler hariç.
  - `Store::create_variant / update_variant(…, expected_version) / trash_variant(…, expected_version, confirm)` — aynı kurallar; ürünün son aktif varyantı çöpe atılırsa `Validation{field:"variant", message:"Ürünün en az bir varyantı olmalı; ürünü silin"}`.

Doğrulama: ad 1–200 karakter, `min_stock ≥ 0`, SKU kiracıda çöpte olmayan kayıtlar arasında tekil.

- [ ] **Step 1: Failing testler**

```rust
#[tokio::test] async fn turkish_search_matches() {
    let (s, ctx) = seeded().await;
    s.create_product(&ctx, np("İpek Eşarp")).await.unwrap();
    s.create_product(&ctx, np("Ispanak Kutusu")).await.unwrap();
    assert_eq!(s.list_products(&ctx, q("ipek")).await.unwrap().len(), 1);
    assert!(s.list_products(&ctx, q("ispanak")).await.unwrap().is_empty());
}
#[tokio::test] async fn stale_update_conflicts() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    s.update_product(&ctx, p.id, patch_name("Kupa 1"), p.row_version).await.unwrap();
    assert!(matches!(s.update_product(&ctx, p.id, patch_name("Kupa 2"), p.row_version).await, Err(DomainError::Conflict)));
}
#[tokio::test] async fn trash_requires_confirm_word() {
    let (s, ctx) = seeded().await;
    let p = s.create_product(&ctx, np("Kupa")).await.unwrap();
    assert!(matches!(s.trash_product(&ctx, p.id, p.row_version, "sil").await, Err(DomainError::Validation{field:"confirm",..})));
    s.trash_product(&ctx, p.id, p.row_version, "SİL").await.unwrap();
    assert!(s.list_products(&ctx, q("kupa")).await.unwrap().is_empty());
}
#[tokio::test] async fn default_variant_created() { /* variants[0].name == "Standart" */ }
#[tokio::test] async fn duplicate_sku_rejected() { /* Validation{field:"sku"} */ }
#[tokio::test] async fn update_writes_audit_before_after() { /* audit_history → [Update, Create]; before/after adları */ }
#[tokio::test] async fn last_variant_cannot_be_trashed_alone() { /* Validation{field:"variant"} */ }
```

- [ ] **Step 2: FAIL** — `cargo test -p stok-db --test catalog`
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(catalog): products and variants with audit, locking, trash`

---

### Task 5: Stok hareketleri (defter)

**Files:**
- Create: `crates/stok-db/src/movements.rs`
- Test: `crates/stok-db/tests/movements.rs`

**Interfaces:**
- Consumes: Task 2 kuralları, `Store`.
- Produces:
  - `Store::record_movement(ctx, NewMovement) -> Result<Movement>` — tek transaction: aynı `idempotency_key` varsa mevcut kaydı döndürür; varyant çöpte/yoksa `NotFound{entity:"variant"}`; firma çöpte/yoksa `NotFound{entity:"firm"}`; delta hesapla, `stock_qty` güncelle, hareketi ad kopyalarıyla ekle.
  - `Store::reverse_movement(ctx, MovementId, reason: String) -> Result<Movement>` — `Reversal`, `delta = -orijinal.delta`, `reverses = Some(id)`, orijinale `reversed_by`. Zaten ters çevrilmişse `Validation{field:"movement"}`; stok negatife düşecekse `InsufficientStock`; varyant çöpteyse `NotFound`.
  - `Store::correct_movement(ctx, MovementId, NewMovement) -> Result<(Movement, Movement)>` — ters kayıt + yeni kayıt, tek transaction.
  - `Store::list_movements(ctx, MovementQuery { variant_id, product_id, firm_id, kind, from_ms, to_ms, include_deleted: bool, limit: u32, before: Option<MovementId> }) -> Result<Vec<MovementView>>` — `MovementView = Movement + user_name + entity_deleted: bool` (ürün/varyant/firma çöpte ya da kalıcı silinmişse `true`). `include_deleted=false` iken `entity_deleted` satırları dönmez.
  - `Store::stock_summary(ctx, StockSummaryQuery { search, from_ms, to_ms }) -> Result<Vec<StockSummaryRow>>` — `StockSummaryRow { variant_id, product_name, variant_name, total_in, total_out, total_adjust, current }`; çöpteki ürünler hariç. `Reversal` satırları ters çevirdiği türün sütununa (eksi olarak) yazılır.

- [ ] **Step 1: Failing testler**

```rust
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
#[tokio::test] async fn reversal_restores_and_keeps_history() { /* in 10, out 4, reverse(out) → 10; 3 kayıt; ikinci reverse → Validation */ }
#[tokio::test] async fn correct_movement_replaces_effect() { /* in 10 → in 7 → stok 7, 3 kayıt */ }
#[tokio::test] async fn ledger_matches_cache() { /* 50 rastgele geçerli hareket → SUM(delta) == stock_qty */ }
#[tokio::test] async fn trashed_variant_rejects_movements_and_hides_history() {
    /* in 2 → trash_product → record_movement NotFound{entity:"variant"};
       list_movements(include_deleted=false) boş; include_deleted=true → 1 satır, entity_deleted=true */
}
#[tokio::test] async fn summary_splits_in_out_adjust() { /* in 10, out 3, adjust→5 → 10, 3, -2, 5 */ }
#[tokio::test] async fn movement_keeps_name_snapshot() { /* hareketten sonra ürün adı değişir → eski hareket eski adı taşır */ }
```

- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(movements): ledger, idempotency, reversal, correction, summary`

---

### Task 6: Firmalar

**Files:**
- Create: `crates/stok-core/src/firm.rs`, `crates/stok-db/src/firms.rs`
- Test: `crates/stok-db/tests/firms.rs`

**Interfaces:**
- Produces: `NewFirm { name, contact: Option<String>, phone: Option<String>, note: Option<String> }`, `FirmPatch`, `Firm { id, name, contact, phone, note, row_version, deleted_at }`; `Store::create_firm / update_firm(…, expected_version) / trash_firm(…, expected_version, confirm) / list_firms(ctx, search: Option<String>)` (çöptekiler hariç). Aynı normalize adlı aktif firma tekrar eklenemez.

- [ ] **Step 1: Failing testler** — `duplicate_firm_name_case_insensitive` ("ABC Matbaa" / "abc matbaa" → `Validation{field:"name"}`), `stale_firm_update_conflicts`, `out_to_trashed_firm_rejected` (`NotFound{entity:"firm"}`), `trash_firm_requires_confirm`.
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(firms): firm CRUD with trash`

---

### Task 7: Kritik stok ve raporlar

**Files:**
- Create: `crates/stok-core/src/report.rs`, `crates/stok-db/src/reports.rs`
- Test: `crates/stok-db/tests/reports.rs`, `report.rs` birim testi

**Interfaces:**
- Produces:
  - `CriticalItem { variant_id, product_name, variant_name, stock_qty, min_stock }`; `Store::critical_stock(ctx)` — `stock_qty <= min_stock AND min_stock > 0`, çöptekiler hariç, `stock_qty - min_stock` artan.
  - `FirmDeliveryRow { firm_id: Option<FirmId>, firm_name, product_name, variant_name, quantity, printed: bool, date_ms, movement_id, entity_deleted: bool }`, `FirmDeliveryTotal { firm_name, printed_qty, unprinted_qty, total_qty }`.
  - `Store::firm_delivery_report(ctx, DeliveryQuery { firm_id, from_ms, to_ms, printed: Option<bool>, include_deleted: bool }) -> Result<(Vec<FirmDeliveryRow>, Vec<FirmDeliveryTotal>)>` — yalnız `Out`, ters çevrilenler hariç; toplamlar filtrelenmiş satırlardan.
  - `pub fn display_name(row_name: &str, entity_deleted: bool) -> String` — silinmişse `"Silinmiş ürün: <ad>"`.
  - `pub fn rows_to_csv(rows: &[FirmDeliveryRow]) -> String` — UTF-8 BOM, ayraç `;`, başlık `Tarih;Firma;Ürün;Varyant;Adet;Baskı`, tarih `GG.AA.YYYY SS:DD` (Europe/Istanbul), `Evet/Hayır`, ürün sütunu `display_name` ile.

- [ ] **Step 1: Failing testler**

```rust
#[tokio::test] async fn critical_includes_equal_threshold() { /* min 5 stok 5 → var; 6 → yok; min 0 → yok; çöpteki → yok */ }
#[tokio::test] async fn report_excludes_reversed_outs() { /* out 4 baskılı + out 2 baskısız, ilki geri alınır → total 2, printed 0 */ }
#[tokio::test] async fn report_hides_deleted_unless_requested() { /* ürün çöpe → include_deleted=false boş, true → entity_deleted=true */ }
#[test] fn csv_has_bom_header_and_deleted_label() {
    let csv = rows_to_csv(&[row("Öz Matbaa", "Kupa", 3, true, true)]);
    assert!(csv.starts_with('\u{FEFF}'));
    assert!(csv.contains("Tarih;Firma;Ürün;Varyant;Adet;Baskı"));
    assert!(csv.contains(";Öz Matbaa;Silinmiş ürün: Kupa;") && csv.contains(";3;Evet"));
}
```

- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(reports): critical stock and firm delivery report with CSV`

---

### Task 8: Çöp kutusu, geri getirme, kalıcı silme

**Files:**
- Create: `crates/stok-core/src/trash.rs`, `crates/stok-db/src/trash.rs`
- Test: `crates/stok-db/tests/trash.rs`

**Interfaces:**
- Consumes: Task 4–7 tabloları, `with_purge_guard`, `check_delete_confirmation`.
- Produces:
  - `pub enum TrashKind { Product, Variant, Firm }`; `TrashItem { kind, id: String, name: String, parent_name: Option<String>, deleted_at: i64, purge_at: i64, stock_qty: i64, row_version: i64 }`
  - `Store::list_trash(ctx) -> Result<Vec<TrashItem>>` — `purge_at = deleted_at + retention_days*86_400_000`; yeniden eskiye. Çöpteki ürünün varyantları ayrıca listelenmez.
  - `Store::restore(ctx, kind, id: &str, expected_version) -> Result<()>` — ürün geri gelince birlikte silinen varyantları da gelir (aynı `deleted_at`); varyant geri gelirken ürünü çöpteyse `Validation{field:"product", message:"Önce ürünü geri getirin"}`; SKU/ad çakışması → `Validation{field:"sku"|"name"}`; audit `Restore`.
  - `Store::purge(ctx, kind, id: &str, confirm: &str, with_history: bool) -> Result<PurgeReport>` — yalnız çöpteki kayıt (değilse `Validation{field:"state", message:"Önce çöp kutusuna taşıyın"}`). `with_history=false`: kayıt(lar) silinir; hareketlerin `*_id` alanları NULL'lanır (`with_purge_guard` içinde, ad kopyaları kalır); audit `Purge` yazılır. `with_history=true`: ilgili hareketler, bu hareketlere bağlı ters kayıtlar ve varlığın `audit_log` kayıtları da silinir; tek bir `Purge` audit satırı ("<ad> geçmişiyle birlikte silindi", kim, ne zaman) kalır. Firma kalıcı silmede `with_history=true` → firmaya ait çıkışlar silinmez, yalnız firma bağlantısı kaldırılır ve stok değişmez (firmanın hareketlerini silmek stoğu bozar); bu durum `PurgeReport.note`'ta belirtilir.
  - `PurgeReport { removed_entities: u32, removed_movements: u32, note: Option<String> }`
  - `Store::purge_expired(ctx, now_ms: i64) -> Result<u32>` — süresi dolan çöp kayıtlarını `with_history=false` ile kalıcı siler.
  - `pub fn validate_retention_days(d: i64) -> Result<(), DomainError>` (1–365).

Not: `with_history=true` ile ürün silindiğinde stok etkisi ortadan kalkar; ürün zaten kalıcı silindiği için özet/kritik listeyi etkilemez. Ürüne ait firma teslim satırları rapordan çıkar (spec §2.12).

- [ ] **Step 1: Failing testler**

```rust
#[tokio::test] async fn restore_brings_back_product_with_stock() {
    /* in 7 → trash_product → list_trash 1 öğe stock_qty 7 → restore → stock_summary current 7 */
}
#[tokio::test] async fn restore_fails_on_sku_clash() { /* trash "KP-1" → yeni ürün "KP-1" → restore Validation{field:"sku"} */ }
#[tokio::test] async fn variant_restore_requires_parent() { /* Validation{field:"product"} */ }
#[tokio::test] async fn purge_requires_confirm_and_trashed_state() { /* "sil" → confirm hatası; çöpte değil → Validation{field:"state"} */ }
#[tokio::test] async fn purge_keeps_history_by_default() {
    /* in 5, out 2 → trash → purge(false) → list_movements(include_deleted=true) 2 satır,
       product_name "Kupa", variant_id None, entity_deleted true; (false) → 0 satır */
}
#[tokio::test] async fn purge_with_history_removes_movements_and_audit() {
    /* purge(true) → list_movements 0; audit_history("product", id) yalnız 1 Purge satırı; PurgeReport.removed_movements == 2 */
}
#[tokio::test] async fn firm_purge_with_history_keeps_stock() { /* firmaya out 3 → trash/purge(true) firm → stok değişmez, note Some */ }
#[tokio::test] async fn expired_items_purged_automatically() {
    /* retention 30; deleted_at = now - 31 gün → purge_expired 1; now - 29 gün → 0 */
}
```

- [ ] **Step 2: FAIL** — `cargo test -p stok-db --test trash`
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(trash): restore, purge with optional history, auto-expiry`

---

### Task 9: Kullanıcılar, roller, tek kullanıcı modu

**Files:**
- Create: `crates/stok-core/src/user.rs`, `crates/stok-db/src/users.rs`
- Test: `crates/stok-db/tests/users.rs`, `user.rs` birim testi

**Interfaces:**
- Produces:
  - `pub enum Role { Admin, Operator }`; `pub enum Permission { ViewAll, ViewDeleted, RecordMovement, CorrectMovement, ManageCatalog, ManageFirms, ManageTrash, ManageUsers, ManageSettings }`; `pub fn can(role: Role, p: Permission) -> bool` — Operator yalnız `ViewAll`, `RecordMovement`.
  - `User { id, username, display_name, role, active, row_version }`
  - `Store::is_initialized()`, `Store::create_first_admin(username, display_name, password) -> Result<(Ctx, User)>` (ikinci kez `Forbidden`), `Store::create_user(ctx, NewUser)`, `Store::authenticate(username, password) -> Result<(Ctx, User)>` (pasif/hatalı → aynı `InvalidCredentials`), `Store::set_password / set_role / set_active(…, expected_version)` (son aktif Admin korunur → `Validation{field:"role"}`), `Store::list_users(ctx)`.
  - `Store::get_setting(ctx, key) -> Result<Option<String>>`, `Store::set_setting(ctx, key, value)`; anahtarlar: `single_user_mode` (`"1"`/`"0"`), `trash_retention_days` (`validate_retention_days` ile).

Kullanıcılar çöp kutusuna gitmez; pasifleştirilir (geçmişte adları kalır).

- [ ] **Step 1: Failing testler** — `operator_permissions` (`can(Operator, ViewDeleted) == false`, `ManageTrash == false`), `password_hashed_not_plain` (`$argon2id$`), `wrong_password_and_inactive_same_error`, `cannot_demote_last_admin`, `first_admin_only_once`, `retention_setting_validated` (0 ve 366 reddedilir).
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(users): auth, roles, settings`

---

### Task 10: Yedekleme / geri yükleme

**Files:**
- Create: `crates/stok-db/src/backup.rs`
- Test: `crates/stok-db/tests/backup.rs`

**Interfaces:**
- Produces:
  - `Store::backup_to(&self, dest: &Path) -> Result<()>` — `VACUUM INTO`; hedef varsa `Validation{field:"path"}`.
  - `pub async fn validate_backup(path: &Path) -> Result<()>` — `application_id` ve migration tablosu; değilse "Bu dosya geçerli bir stok yedeği değil".
  - `Store::restore_from(self, src: &Path, live_db: &Path) -> Result<Store>` — doğrula → kapat → mevcut dosyayı `stok.db.before-restore-<ms>` olarak sakla → kopyala → aç.
  - `Store::auto_backup(&self, dir: &Path, keep: usize) -> Result<Option<PathBuf>>` — günde bir, en yeni 14 tutulur (yalnız uygulamanın ürettiği `stok-YYYYMMDD.db` dosyaları döndürülür/silinir).

- [ ] **Step 1: Failing testler** — `backup_roundtrip_preserves_stock_and_trash`, `restore_rejects_foreign_sqlite_file`, `restore_keeps_previous_copy`, `auto_backup_once_per_day_and_rotates`.
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(backup): backup, restore, daily auto-backup`

---

### Task 11: Tauri kabuğu, komutlar, yetki, görünürlük

**Files:**
- Create: `src-tauri/` (`cargo tauri init`), `src-tauri/src/{main,state,error,visibility}.rs`, `src-tauri/src/commands/*.rs`, `src-tauri/capabilities/default.json`
- Test: `src-tauri/src/commands/tests.rs`

**Interfaces:**
- Consumes: tüm `Store` metotları.
- Produces:
  - `AppState { store: RwLock<Store>, session: Mutex<Option<(Ctx, User)>>, paths: AppPaths }`; `AppPaths { db, backups, images }` (`app_data_dir` altında).
  - `ApiError { code: String, message: String }` + `From<DomainError>`.
  - `require(&state, Permission) -> Result<(Ctx, Role), ApiError>`; `visibility::include_deleted(role) -> bool` (= `can(role, ViewDeleted)`). Liste/rapor komutları `include_deleted`'ı istemciden **almaz**, rolden belirler.
  - Komutlar: `app_status` → `{ initialized, single_user_mode, user }`, `setup_first_admin`, `login`, `logout`, `list_products`, `create_product`, `update_product`, `trash_product(id, expected_version, confirm)`, `create_variant`, `update_variant`, `trash_variant(…, confirm)`, `set_product_image(product_id, source_path)` (png/jpg/webp, ≤ 5 MB, `images/<id>.<ext>`), `record_movement`, `reverse_movement`, `correct_movement`, `list_movements`, `stock_summary`, `list_firms`, `create_firm`, `update_firm`, `trash_firm(…, confirm)`, `critical_stock`, `firm_delivery_report`, `export_delivery_csv(query, dest_path)`, `audit_history`, `list_trash`, `restore_item(kind, id, expected_version)`, `purge_item(kind, id, confirm, with_history)`, `list_users`, `create_user`, `set_user_role`, `set_user_active`, `set_password`, `get_settings`, `set_single_user_mode`, `set_trash_retention_days`, `backup_now(dest_path)`, `restore_backup(src_path)`.
  - Dönen görünüm satırlarında ad alanları `display_name()` ile etiketlenir.
  - DTO'lar `#[derive(TS)] #[ts(export, export_to = "../../ui/src/lib/bindings/")]`.
  - Açılış: `Store::open` → `auto_backup` → `purge_expired` → `single_user_mode` ise son Admin ile oturum. Uygulama açıkken her 6 saatte bir `purge_expired`.

- [ ] **Step 1: Failing testler** — `operator_cannot_trash_or_purge` (`forbidden`), `operator_never_sees_deleted_rows` (çöpe atılmış ürünün hareketi Operator'da dönmez, Admin'de "Silinmiş ürün: Kupa"), `commands_require_session`, `image_rejects_non_image_and_large_files`, `single_user_mode_auto_logs_in`, `startup_purges_expired`.
- [ ] **Step 2: FAIL** — `cargo test -p stok-takip-app`
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**; kökte `cargo test --workspace` PASS; `cargo clippy --workspace -- -D warnings` temiz.
- [ ] **Step 5: Tip üretimi** — `cargo test export_bindings` → `ui/src/lib/bindings/*.ts`.
- [ ] **Step 6: Commit** `feat(app): tauri commands, permissions, visibility, bindings`

---

### Task 12: Arayüz iskeleti, tema, oturum

**Files:**
- Create: `ui/` (SvelteKit + adapter-static, `ssr=false`), `ui/src/lib/api.ts`, `ui/src/lib/i18n/tr.ts`, `ui/src/lib/components/{AppShell,Sidebar,Toast,ConfirmDialog,DeleteConfirmDialog,DataTable,EmptyState}.svelte`, `ui/src/routes/{+layout.svelte,kurulum,giris}/…`
- Test: `ui/src/lib/api.test.ts`, `ui/src/lib/components/DeleteConfirmDialog.test.ts`

**Interfaces:**
- Consumes: Task 11 komutları, `bindings/`.
- Produces:
  - `api.ts`: her komut için tipli sarmalayıcı; hatalar `ApiError` fırlatır, `Toast` `message` gösterir. Hareket formu açılışında `idempotency_key = crypto.randomUUID()`.
  - `DeleteConfirmDialog` props: `title: string`, `description: string`, `showHistoryOption: boolean`, `onConfirm(confirm: string, withHistory: boolean)`. "Sil" butonu yalnız giriş `trim()` sonrası tam `"SİL"` iken aktif; Türkçe klavyesi olmayanlar için alanın yanında "SİL kelimesini kopyala" butonu.

Tasarım: uygulayıcı önce `frontend-design` skill'ini yükler. Nötr palet + tek vurgu rengi; açık/koyu tema; sol menü: Özet, Ürünler, Hareketler, Firmalar, Raporlar, Çöp Kutusu (Admin), Kullanıcılar (Admin), Ayarlar. Klavye: `Ctrl/Cmd+N` yeni hareket, `Ctrl/Cmd+F` arama, `Esc` kapat (macOS'ta Cmd).

- [ ] **Step 1: Failing testler** — `api` hata dönüşümü; `DeleteConfirmDialog`: "sil"/"SIL" ile buton pasif, "SİL" ile aktif; `showHistoryOption=false` iken onay kutusu yok; onayda `onConfirm("SİL", false)`.
- [ ] **Step 2: FAIL** — `cd ui && npx vitest run`
- [ ] **Step 3: Uygula**: ilk açılışta `/kurulum`, sonra `/giris`; `app_status`'a göre yönlendirme.
- [ ] **Step 4: PASS**; `npm run check` hatasız.
- [ ] **Step 5: Commit** `feat(ui): shell, theme, auth flow, delete confirmation`

---

### Task 13: Ürünler, varyantlar, görsel/liste görünümü

**Files:**
- Create: `ui/src/routes/urunler/+page.svelte`, `ui/src/routes/urunler/[id]/+page.svelte`, `ui/src/lib/components/{ProductForm,VariantTable,ProductGrid,HistoryPanel}.svelte`
- Test: `ProductForm.test.ts`, `ProductGrid.test.ts`, `HistoryPanel.test.ts`

**Interfaces:**
- Consumes: `api` ürün/varyant/görsel/geçmiş fonksiyonları, `DeleteConfirmDialog`.
- Produces: `HistoryPanel` (props `entity: "product"|"variant"|"firm"`, `id: string`) — Task 14'te firmalar için kullanılır.

Davranış: liste/ızgara geçişi (tercih `localStorage`, try/catch); görselsiz ürün baş harf rozeti; 300 ms gecikmeli arama; ürün detayında varyant tablosu (Ad, SKU, Kritik eşik, Mevcut) + "Geçmiş" sekmesi. Silme `DeleteConfirmDialog` (`showHistoryOption=false`) ile → çöp kutusu; başarıda toast "Çöp kutusuna taşındı — Geri al" (Geri al → `restore_item`). Operator'da düzenle/sil gizli.

- [ ] **Step 1: Failing testler** — boş adla kaydet pasif; `min_stock` negatif girilemez; görselsiz ızgarada baş harf; `Conflict` sonrası "Yenile" butonu; silme toast'ındaki "Geri al" `api.restoreItem`'ı çağırır.
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(ui): products, variants, grid/list, history`

---

### Task 14: Hareketler, özet, firmalar, raporlar, çöp kutusu, kullanıcılar, ayarlar

**Files:**
- Create: `ui/src/routes/{+page.svelte,hareketler,firmalar,raporlar,cop-kutusu,kullanicilar,ayarlar}/…`, `ui/src/lib/components/{MovementDialog,StockSummaryTable,CriticalBanner,TrashTable}.svelte`
- Test: `MovementDialog.test.ts`, `StockSummaryTable.test.ts`, `TrashTable.test.ts`

**Interfaces:**
- Consumes: Task 12–13 bileşenleri, ilgili `api` fonksiyonları.

Davranış:
- **Özet:** `StockSummaryTable` yalnız 5 sütun; tarih filtresi; üstte `CriticalBanner`; eşik altı satırlar vurgulu.
- **MovementDialog:** Giriş / Çıkış / Sayım-Düzeltme sekmeleri; Çıkış'ta firma (opsiyonel) + "Baskı yapıldı" anahtarı; Düzeltme'de anlık "Fark" önizlemesi; gönderirken buton kilitli.
- **Hareketler:** filtreli defter, sonsuz kaydırma; Admin için "Düzelt" ve "Geri al" (sebep zorunlu); ters çevrilmiş satırlar üstü çizili; silinmiş varlık satırları (yalnız Admin'e gelir) soluk + "Silinmiş ürün: <ad>".
- **Firmalar:** CRUD + `HistoryPanel` + çöpe taşıma.
- **Raporlar:** firma teslim raporu (firma, tarih, baskı filtresi), baskılı/baskısız/toplam alt toplamları, "CSV olarak kaydet".
- **Çöp Kutusu (Admin):** `TrashTable` sütunları Tür, Ad, Silinme tarihi, Kalıcı silinme (kalan gün), Stok; satır eylemleri "Geri getir" ve "Kalıcı sil" (`DeleteConfirmDialog`, `showHistoryOption=true`, etiket "Geçmişiyle birlikte sil (hareketler ve değişiklik kayıtları da silinir)"). Üstte "Süresi dolanlar X gün sonra otomatik silinir".
- **Kullanıcılar (Admin):** ekle, rol, pasifleştir, parola sıfırla.
- **Ayarlar:** tek kullanıcı modu, çöp kutusu süresi (1–365 gün), "Şimdi yedekle", "Yedekten geri yükle" (`DeleteConfirmDialog` benzeri iki aşamalı onay), yedek klasörünü aç.

- [ ] **Step 1: Failing testler** — Çıkış'ta stoktan fazla miktar uyarısı ve `insufficient_stock` toast'ı; Düzeltme önizlemesi `sayılan − mevcut`; çift tıklamada aynı `idempotency_key`; `StockSummaryTable` başlıkları tam `["Ürün / Varyant","Giren","Çıkan","Düzeltme","Mevcut"]`; `TrashTable` kalan gün `ceil((purge_at − now)/gün)`; "Kalıcı sil" onayında `api.purgeItem(kind, id, "SİL", true)` (kutu işaretli).
- [ ] **Step 2: FAIL**
- [ ] **Step 3: Uygula**
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(ui): movements, summary, firms, reports, trash, users, settings`

---

### Task 15: Çapraz platform paketleme ve uçtan uca doğrulama

**Files:**
- Modify: `src-tauri/tauri.conf.json` (ürün adı "Stok Takip", `identifier` `com.<kullanıcı>.stoktakip`, hedefler: Windows `nsis`, macOS `dmg` + `app`, Linux `deb` + `appimage`; ikon seti)
- Create: `.github/workflows/release.yml`, `docs/kurulum.md`, `docs/test-senaryosu.md`

- [ ] **Step 1: `release.yml`:** `tauri-apps/tauri-action` ile matris `windows-latest`, `macos-latest` (universal: `--target universal-apple-darwin`), `ubuntu-22.04`; her iş önce `cargo test --workspace` ve `npx vitest run` çalıştırır, sonra paketler; etiket (`v*`) push'unda taslak sürüm oluşturur. macOS imzalama/notarization değişkenleri (`APPLE_CERTIFICATE`, `APPLE_ID`, `APPLE_TEAM_ID` …) opsiyonel; yoksa imzasız paket üretir.
- [ ] **Step 2: Etiket push → üç platformun paketleri taslak sürümde görünür.**
- [ ] **Step 3: Her platformda `docs/test-senaryosu.md`:** kurulum → ilk yönetici → ürün + 2 varyant → giriş 10 → baskılı çıkış 4 → baskısız çıkış 2 → sayım 3 → özet: Giren 10, Çıkan 6, Düzeltme −1, Mevcut 3 → kritik eşik 3 → uyarı → CSV Türkçe karakterlerle açılır → ürünü sil ("sil" yazınca buton pasif, "SİL" ile aktif) → Çöp Kutusu'nda görünür → geri getir, stok 3 → tekrar sil → kalıcı sil (geçmiş korunur) → Admin hareketlerde "Silinmiş ürün: <ad>" görür, Operatör görmez → başka ürünü "geçmişiyle birlikte" kalıcı sil → hareketleri yok → yedek al / hareket ekle / geri yükle → hareket yok → kapat/aç: veriler duruyor.
- [ ] **Step 4: Beklenen:** tüm adımlar değerlerle eşleşir; kurulum boyutu ve açılış süresi platform başına `docs/test-senaryosu.md`'ye yazılır.
- [ ] **Step 5: Commit** `chore: cross-platform packaging, CI, e2e checklist`

---

## Sonraki planlar (bu plana dahil değil)

- **Aşama 2 — Site entegrasyonu:** `stok-sync` crate'i; `trait SiteConnector { push_products(&[ProductPayload]); pull_orders(cursor) -> (Vec<RemoteOrder>, cursor); list_remote_products() }`; WooCommerce REST API bağlayıcısı; Vitrin için WordPress köprü eklentisi (Vitrin'e dokunmadan); genel JSON protokolü. **Sitede stok gösterilmez.** Program → site: ürün/varyant bilgisi. Site → program (Vitrin sipariş almaya başladığında): siparişler "Gelen siparişler" listesine düşer; buradan yeni stok girişi veya yeni ürün siparişi oluşturulur. Eşleştirme `external_refs` tablosunda.
- **Aşama 3 — Sunucu:** PostgreSQL deposu (aynı SQL + tetikleyici karşılıkları), hesap/kiracı yönetimi, çoklu cihaz, bulut yedek.
