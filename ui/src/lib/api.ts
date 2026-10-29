import { invoke } from '@tauri-apps/api/core';
import type { AppStatus } from './bindings/AppStatus';
import type { AuditEntry } from './bindings/AuditEntry';
import type { CriticalItem } from './bindings/CriticalItem';
import type { DeliveryQuery } from './bindings/DeliveryQuery';
import type { DeliveryReport } from './bindings/DeliveryReport';
import type { Firm } from './bindings/Firm';
import type { FirmPatch } from './bindings/FirmPatch';
import type { Movement } from './bindings/Movement';
import type { MovementQuery } from './bindings/MovementQuery';
import type { MovementView } from './bindings/MovementView';
import type { NewFirm } from './bindings/NewFirm';
import type { NewMovement } from './bindings/NewMovement';
import type { NewProduct } from './bindings/NewProduct';
import type { NewUser } from './bindings/NewUser';
import type { NewVariant } from './bindings/NewVariant';
import type { Product } from './bindings/Product';
import type { ProductPatch } from './bindings/ProductPatch';
import type { ProductWithVariants } from './bindings/ProductWithVariants';
import type { PurgeReport } from './bindings/PurgeReport';
import type { Role } from './bindings/Role';
import type { Settings } from './bindings/Settings';
import type { StockSummaryQuery } from './bindings/StockSummaryQuery';
import type { StockSummaryRow } from './bindings/StockSummaryRow';
import type { TrashItem } from './bindings/TrashItem';
import type { TrashKind } from './bindings/TrashKind';
import type { UpdateInfo } from './bindings/UpdateInfo';
import type { User } from './bindings/User';
import type { Variant } from './bindings/Variant';
import type { VariantPatch } from './bindings/VariantPatch';

export class ApiError extends Error {
	constructor(
		public code: string,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export function toApiError(e: unknown): ApiError {
	if (e instanceof ApiError) return e;
	if (e && typeof e === 'object' && 'code' in e && 'message' in e) {
		return new ApiError(String(e.code), String(e.message));
	}
	if (typeof e === 'string') return new ApiError('unknown', e);
	return new ApiError('unknown', 'İşlem tamamlanamadı');
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(cmd, args);
	} catch (e) {
		throw toApiError(e);
	}
}

const movementQuery = (q: Partial<MovementQuery>): MovementQuery => ({
	variant_id: null,
	product_id: null,
	firm_id: null,
	kind: null,
	from_ms: null,
	to_ms: null,
	limit: 200,
	before: null,
	...q
});

const deliveryQuery = (q: Partial<DeliveryQuery>): DeliveryQuery => ({
	firm_id: null,
	from_ms: null,
	to_ms: null,
	printed: null,
	...q
});

// Tauri, Rust'taki snake_case parametreleri JS'de camelCase bekler.
export const api = {
	appStatus: () => call<AppStatus>('app_status'),
	setupFirstAdmin: (username: string, displayName: string, password: string) =>
		call<AppStatus>('setup_first_admin', { username, displayName, password }),
	login: (username: string, password: string) => call<AppStatus>('login', { username, password }),
	logout: () => call<void>('logout'),

	listUsers: () => call<User[]>('list_users'),
	createUser: (user: NewUser) => call<User>('create_user', { user }),
	setUserRole: (id: string, role: Role, expectedVersion: number) =>
		call<User>('set_user_role', { id, role, expectedVersion }),
	setUserActive: (id: string, active: boolean, expectedVersion: number) =>
		call<User>('set_user_active', { id, active, expectedVersion }),
	setPassword: (id: string, password: string, expectedVersion: number) =>
		call<User>('set_password', { id, password, expectedVersion }),

	listProducts: (search?: string) => call<ProductWithVariants[]>('list_products', { query: { search: search || null } }),
	getProduct: (id: string) => call<ProductWithVariants>('get_product', { id }),
	createProduct: (product: NewProduct) => call<ProductWithVariants>('create_product', { product }),
	updateProduct: (id: string, patch: Partial<ProductPatch>, expectedVersion: number) =>
		call<Product>('update_product', {
			id,
			patch: { name: null, sku: null, description: null, image_path: null, ...patch },
			expectedVersion
		}),
	trashProduct: (id: string, expectedVersion: number, confirm: string) =>
		call<void>('trash_product', { id, expectedVersion, confirm }),
	createVariant: (variant: NewVariant) => call<Variant>('create_variant', { variant }),
	updateVariant: (id: string, patch: Partial<VariantPatch>, expectedVersion: number) =>
		call<Variant>('update_variant', { id, patch: { name: null, sku: null, min_stock: null, ...patch }, expectedVersion }),
	trashVariant: (id: string, expectedVersion: number, confirm: string) =>
		call<void>('trash_variant', { id, expectedVersion, confirm }),
	setProductImage: (productId: string, sourcePath: string) =>
		call<Product>('set_product_image', { productId, sourcePath }),
	getProductImage: (productId: string) => call<string | null>('get_product_image', { productId }),
	auditHistory: (entity: string, entityId: string) => call<AuditEntry[]>('audit_history', { entity, entityId }),

	recordMovement: (movement: NewMovement) => call<Movement>('record_movement', { movement }),
	reverseMovement: (id: string, reason: string) => call<Movement>('reverse_movement', { id, reason }),
	correctMovement: (id: string, movement: NewMovement) =>
		call<[Movement, Movement]>('correct_movement', { id, movement }),
	listMovements: (q: Partial<MovementQuery> = {}) => call<MovementView[]>('list_movements', { query: movementQuery(q) }),
	stockSummary: (q: Partial<StockSummaryQuery> = {}) =>
		call<StockSummaryRow[]>('stock_summary', { query: { search: null, from_ms: null, to_ms: null, ...q } }),

	listFirms: (search?: string) => call<Firm[]>('list_firms', { search: search || null }),
	createFirm: (firm: NewFirm) => call<Firm>('create_firm', { firm }),
	updateFirm: (id: string, patch: Partial<FirmPatch>, expectedVersion: number) =>
		call<Firm>('update_firm', { id, patch: { name: null, contact: null, phone: null, note: null, ...patch }, expectedVersion }),
	trashFirm: (id: string, expectedVersion: number, confirm: string) =>
		call<void>('trash_firm', { id, expectedVersion, confirm }),

	criticalStock: () => call<CriticalItem[]>('critical_stock'),
	firmDeliveryReport: (q: Partial<DeliveryQuery> = {}) =>
		call<DeliveryReport>('firm_delivery_report', { query: deliveryQuery(q) }),
	exportDeliveryCsv: (q: Partial<DeliveryQuery>, destPath: string) =>
		call<number>('export_delivery_csv', { query: deliveryQuery(q), destPath }),

	listTrash: () => call<TrashItem[]>('list_trash'),
	restoreItem: (kind: TrashKind, id: string, expectedVersion: number) =>
		call<void>('restore_item', { kind, id, expectedVersion }),
	purgeItem: (kind: TrashKind, id: string, confirm: string, withHistory: boolean) =>
		call<PurgeReport>('purge_item', { kind, id, confirm, withHistory }),

	getSettings: () => call<Settings>('get_settings'),
	setSingleUserMode: (enabled: boolean) => call<void>('set_single_user_mode', { enabled }),
	setTrashRetentionDays: (days: number) => call<void>('set_trash_retention_days', { days }),
	backupNow: (destPath: string) => call<void>('backup_now', { destPath }),
	restoreBackup: (srcPath: string) => call<AppStatus>('restore_backup', { srcPath }),
	openBackupsDir: () => call<void>('open_backups_dir'),

	checkUpdate: () => call<UpdateInfo>('check_update'),
	installUpdate: () => call<void>('install_update')
};

export type Api = typeof api;
