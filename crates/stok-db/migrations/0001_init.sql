-- Stok Takip şeması. Söz dizimi SQLite ve PostgreSQL'de ortak tutulur.
-- Zamanlar: UTC epoch ms. Kimlikler: UUID v7 metin.

CREATE TABLE tenants (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    username TEXT NOT NULL,
    username_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'operator')),
    active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);
CREATE UNIQUE INDEX users_username ON users (tenant_id, username_key);

CREATE TABLE products (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL,
    search_key TEXT NOT NULL,
    sku TEXT NULL,
    sku_key TEXT NULL,
    description TEXT NULL,
    image_path TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);
CREATE INDEX products_search ON products (tenant_id, search_key);
CREATE INDEX products_sku ON products (tenant_id, sku_key);

CREATE TABLE variants (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    name TEXT NOT NULL,
    search_key TEXT NOT NULL,
    sku TEXT NULL,
    sku_key TEXT NULL,
    min_stock INTEGER NOT NULL DEFAULT 0 CHECK (min_stock >= 0),
    stock_qty INTEGER NOT NULL DEFAULT 0 CHECK (stock_qty >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);
CREATE INDEX variants_product ON variants (product_id);
CREATE INDEX variants_sku ON variants (tenant_id, sku_key);

CREATE TABLE firms (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL,
    search_key TEXT NOT NULL,
    contact TEXT NULL,
    phone TEXT NULL,
    note TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);
CREATE INDEX firms_search ON firms (tenant_id, search_key);

-- Hareket defteri: yalnız INSERT. variant/product/firm kimliklerinde FK yok,
-- kalıcı silmeden sonra NULL olabilir; *_name sütunları ad kopyasıdır.
CREATE TABLE stock_movements (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    variant_id TEXT NULL,
    product_id TEXT NULL,
    product_name TEXT NOT NULL,
    variant_name TEXT NOT NULL,
    firm_id TEXT NULL,
    firm_name TEXT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('in', 'out', 'adjust', 'reversal')),
    delta INTEGER NOT NULL,
    quantity INTEGER NOT NULL,
    printed INTEGER NULL,
    note TEXT NULL,
    source TEXT NOT NULL CHECK (source IN ('local', 'site')),
    idempotency_key TEXT NOT NULL,
    reverses TEXT NULL,
    reversed_by TEXT NULL,
    created_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL,
    UNIQUE (tenant_id, idempotency_key)
);
CREATE INDEX movements_variant ON stock_movements (variant_id);
CREATE INDEX movements_product ON stock_movements (product_id);
CREATE INDEX movements_firm ON stock_movements (firm_id);
CREATE INDEX movements_created ON stock_movements (tenant_id, created_at);

CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    before_json TEXT NULL,
    after_json TEXT NULL,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL
);
CREATE INDEX audit_entity ON audit_log (tenant_id, entity, entity_id);

CREATE TABLE app_settings (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL,
    UNIQUE (tenant_id, key)
);

-- Aşama 2 (site eşleştirme) için.
CREATE TABLE external_refs (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    connector TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    row_version INTEGER NOT NULL DEFAULT 1,
    deleted_at INTEGER NULL,
    UNIQUE (tenant_id, connector, entity, remote_id)
);

-- Yalnız kalıcı silme transaction'ı içinde dolu olur.
CREATE TABLE purge_guard (id INTEGER PRIMARY KEY);

-- ===== Tetikleyiciler (SQLite). Aşama 3'te PostgreSQL karşılıkları yazılır. =====

CREATE TRIGGER stock_movements_no_delete BEFORE DELETE ON stock_movements
WHEN NOT EXISTS (SELECT 1 FROM purge_guard)
BEGIN SELECT RAISE(ABORT, 'append_only'); END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
WHEN NOT EXISTS (SELECT 1 FROM purge_guard)
BEGIN SELECT RAISE(ABORT, 'append_only'); END;

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN SELECT RAISE(ABORT, 'append_only'); END;

-- Tek izin: reversed_by NULL'dan bir değere (diğer sütunlar aynı kalarak).
-- Kalıcı silmede (purge_guard dolu) kimlik sütunları NULL'lanabilir.
CREATE TRIGGER stock_movements_no_update BEFORE UPDATE ON stock_movements
WHEN NOT EXISTS (SELECT 1 FROM purge_guard)
 AND NOT (
    OLD.reversed_by IS NULL AND NEW.reversed_by IS NOT NULL
    AND NEW.id IS OLD.id AND NEW.tenant_id IS OLD.tenant_id
    AND NEW.variant_id IS OLD.variant_id AND NEW.product_id IS OLD.product_id
    AND NEW.product_name IS OLD.product_name AND NEW.variant_name IS OLD.variant_name
    AND NEW.firm_id IS OLD.firm_id AND NEW.firm_name IS OLD.firm_name
    AND NEW.kind IS OLD.kind AND NEW.delta IS OLD.delta AND NEW.quantity IS OLD.quantity
    AND NEW.printed IS OLD.printed AND NEW.note IS OLD.note AND NEW.source IS OLD.source
    AND NEW.idempotency_key IS OLD.idempotency_key AND NEW.reverses IS OLD.reverses
    AND NEW.created_at IS OLD.created_at AND NEW.created_by IS OLD.created_by
 )
BEGIN SELECT RAISE(ABORT, 'append_only'); END;
