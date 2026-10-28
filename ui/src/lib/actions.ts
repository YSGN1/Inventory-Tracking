import { api } from './api';
import type { Firm } from './bindings/Firm';
import type { Product } from './bindings/Product';
import type { TrashKind } from './bindings/TrashKind';
import { tr } from './i18n/tr';
import { attempt, bumpRefresh, toast } from './state.svelte';

/**
 * Çöpe taşı ve "Geri al" sunan bildirim göster. Çöpe taşıma `row_version`'ı bir artırır;
 * geri alma bu yeni sürümle yapılır.
 */
async function trashWithUndo(kind: TrashKind, id: string, version: number, run: () => Promise<void>) {
	await run();
	bumpRefresh();
	toast(tr.del.movedToTrash, 'success', {
		label: tr.common.undo,
		run: async () => {
			const ok = await attempt(api.restoreItem(kind, id, version + 1));
			if (ok !== undefined) {
				toast(tr.trash.restored);
				bumpRefresh();
			}
		}
	});
}

export const trashProductWithUndo = (p: Product, confirm: string) =>
	trashWithUndo('product', p.id, p.row_version, () => api.trashProduct(p.id, p.row_version, confirm));

export const trashFirmWithUndo = (f: Firm, confirm: string) =>
	trashWithUndo('firm', f.id, f.row_version, () => api.trashFirm(f.id, f.row_version, confirm));

export const trashVariantWithUndo = (v: { id: string; row_version: number }, confirm: string) =>
	trashWithUndo('variant', v.id, v.row_version, () => api.trashVariant(v.id, v.row_version, confirm));
