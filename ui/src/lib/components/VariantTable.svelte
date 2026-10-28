<script lang="ts">
	import { api, toApiError } from '$lib/api';
	import { trashVariantWithUndo } from '$lib/actions';
	import type { Variant } from '$lib/bindings/Variant';
	import { fmtNum } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { bumpRefresh, isConflict, openMovement, toast } from '$lib/state.svelte';
	import DataTable from './DataTable.svelte';
	import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';
	import Modal from './Modal.svelte';

	let { productId, variants, admin }: { productId: string; variants: Variant[]; admin: boolean } = $props();

	// Düzenlenen/eklenen varyant; id yoksa yeni.
	let editing = $state<{ id: string | null; name: string; sku: string; min_stock: number; row_version: number } | null>(null);
	let deleting = $state<Variant | null>(null);
	let conflict = $state(false);
	let busy = $state(false);

	const valid = $derived(!!editing && editing.name.trim().length > 0 && Number(editing.min_stock) >= 0);

	function edit(v: Variant | null) {
		conflict = false;
		editing = v
			? { id: v.id, name: v.name, sku: v.sku ?? '', min_stock: v.min_stock, row_version: v.row_version }
			: { id: null, name: '', sku: '', min_stock: 0, row_version: 0 };
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!editing || !valid || busy) return;
		busy = true;
		const min = Number(editing.min_stock);
		try {
			if (editing.id) {
				await api.updateVariant(editing.id, { name: editing.name.trim(), sku: editing.sku.trim(), min_stock: min }, editing.row_version);
			} else {
				await api.createVariant({ product_id: productId, name: editing.name.trim(), sku: editing.sku.trim() || null, min_stock: min });
			}
			editing = null;
			toast(tr.products.saved);
			bumpRefresh();
		} catch (err) {
			if (isConflict(err)) conflict = true;
			else toast(toApiError(err).message, 'error');
		} finally {
			busy = false;
		}
	}
</script>

<DataTable headers={[tr.common.name, tr.common.sku, tr.products.minStock, tr.products.current, '']} numeric={[2, 3]}>
	{#each variants as v (v.id)}
		<tr>
			<td>{v.name}</td>
			<td class="text-muted">{v.sku ?? ''}</td>
			<td class="num">{v.min_stock > 0 ? fmtNum(v.min_stock) : '—'}</td>
			<td class="num">
				<span class="stock" class:low={v.min_stock > 0 && v.stock_qty <= v.min_stock}>{fmtNum(v.stock_qty)}</span>
			</td>
			<td class="actions">
				<button class="btn btn-sm" onclick={() => openMovement(v.id)}>{tr.nav.newMovement}</button>
				{#if admin}
					<button class="btn btn-sm btn-ghost" onclick={() => edit(v)}>{tr.common.edit}</button>
					<button class="btn btn-sm btn-ghost text-danger" onclick={() => (deleting = v)}>{tr.common.delete}</button>
				{/if}
			</td>
		</tr>
	{/each}
</DataTable>
{#if admin}
	<button class="btn btn-sm mt-3" onclick={() => edit(null)}>{tr.products.addVariant}</button>
{/if}

<Modal open={!!editing} title={editing?.id ? tr.common.edit : tr.products.addVariant} onClose={() => (editing = null)}>
	{#if editing}
		<form class="grid gap-3" onsubmit={save}>
			<label class="field">
				<span>{tr.products.variantName}</span>
				<input class="input" bind:value={editing.name} maxlength="200" />
			</label>
			<div class="grid grid-cols-2 gap-3">
				<label class="field">
					<span>{tr.common.sku}</span>
					<input class="input" bind:value={editing.sku} maxlength="64" />
				</label>
				<label class="field">
					<span>{tr.products.minStock}</span>
					<input class="input num" type="number" min="0" step="1" bind:value={editing.min_stock} aria-invalid={Number(editing.min_stock) < 0} />
				</label>
			</div>
			{#if Number(editing.min_stock) < 0}<p class="text-danger text-[13px]">{tr.products.minStockNegative}</p>{/if}
			{#if conflict}
				<p class="text-accent-ink bg-accent-soft rounded-md px-3 py-2" role="alert">{tr.common.conflictHint}</p>
			{/if}
			<div class="flex justify-end gap-2">
				<button type="button" class="btn" onclick={() => (editing = null)}>{tr.common.cancel}</button>
				<button class="btn btn-primary" disabled={!valid || busy}>{tr.common.save}</button>
			</div>
		</form>
	{/if}
</Modal>

<DeleteConfirmDialog
	open={!!deleting}
	title={tr.products.deleteVariant(deleting?.name ?? '')}
	description={tr.products.deleteProductDesc}
	onCancel={() => (deleting = null)}
	onConfirm={async (confirm) => {
		const v = deleting;
		if (!v) return;
		try {
			await trashVariantWithUndo(v, confirm);
			deleting = null;
		} catch (err) {
			toast(toApiError(err).message, 'error');
		}
	}}
/>

<style>
	.actions {
		text-align: right;
		white-space: nowrap;
	}
	.stock {
		font-size: 16px;
		font-weight: 600;
	}
	.stock.low {
		color: var(--accent-ink);
		background: var(--accent-soft);
		padding: 1px 6px;
		border-radius: 4px;
	}
</style>
