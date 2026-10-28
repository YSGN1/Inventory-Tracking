<script lang="ts">
	import { untrack } from 'svelte';
	import type { VariantDraft } from '$lib/bindings/VariantDraft';
	import { tr } from '$lib/i18n/tr';
	import { isConflict, toast } from '$lib/state.svelte';
	import { toApiError } from '$lib/api';

	export type ProductFormData = { name: string; sku: string; description: string; variants: VariantDraft[] };

	let {
		initial = null,
		onSave,
		onCancel,
		onRefresh
	}: {
		/** Düzenleme modunda mevcut değerler; yoksa yeni ürün (varyantlarla). */
		initial?: { name: string; sku: string | null; description: string | null } | null;
		onSave: (data: ProductFormData) => Promise<void> | void;
		onCancel: () => void;
		onRefresh?: () => void;
	} = $props();

	// Form, açıldığı andaki değerlerin kopyasıyla çalışır; üst bileşen yenilemede yeniden oluşturur.
	let name = $state(untrack(() => initial?.name ?? ''));
	let sku = $state(untrack(() => initial?.sku ?? ''));
	let description = $state(untrack(() => initial?.description ?? ''));
	let drafts = $state<{ name: string; sku: string; min_stock: number }[]>([]);
	let busy = $state(false);
	let conflict = $state(false);

	const negativeMin = $derived(drafts.some((d) => Number(d.min_stock) < 0 || Number.isNaN(Number(d.min_stock))));
	const blankVariant = $derived(drafts.some((d) => !d.name.trim()));
	const valid = $derived(name.trim().length > 0 && !negativeMin && !blankVariant);

	async function submit(e?: SubmitEvent) {
		e?.preventDefault();
		if (!valid || busy) return;
		busy = true;
		conflict = false;
		try {
			await onSave({
				name: name.trim(),
				sku: sku.trim(),
				description: description.trim(),
				variants: drafts.map((d) => ({ name: d.name.trim(), sku: d.sku.trim() || null, min_stock: Number(d.min_stock) || 0 }))
			});
		} catch (err) {
			if (isConflict(err)) conflict = true;
			else toast(toApiError(err).message, 'error');
		} finally {
			busy = false;
		}
	}
</script>

<form class="grid gap-3" onsubmit={submit}>
	<label class="field">
		<span>{tr.common.name}</span>
		<input class="input" bind:value={name} maxlength="200" data-autofocus />
	</label>
	<div class="grid grid-cols-2 gap-3">
		<label class="field">
			<span>{tr.common.sku}</span>
			<input class="input" bind:value={sku} maxlength="64" autocomplete="off" />
		</label>
	</div>
	<label class="field">
		<span>{tr.products.description}</span>
		<textarea class="input" rows="2" bind:value={description} maxlength="2000"></textarea>
	</label>

	{#if !initial}
		<fieldset class="grid gap-2">
			<legend class="text-[12.5px] font-medium text-muted mb-1">{tr.products.variants}</legend>
			{#each drafts as d, i (i)}
				<div class="variant">
					<label class="field">
						<span>{tr.products.variantName}</span>
						<input class="input" bind:value={d.name} maxlength="200" />
					</label>
					<label class="field">
						<span>{tr.common.sku}</span>
						<input class="input" bind:value={d.sku} maxlength="64" />
					</label>
					<label class="field">
						<span>{tr.products.minStock}</span>
						<input class="input num" type="number" min="0" step="1" bind:value={d.min_stock} aria-invalid={Number(d.min_stock) < 0} />
					</label>
					<button type="button" class="btn btn-sm btn-ghost self-end" aria-label="Varyantı kaldır" onclick={() => drafts.splice(i, 1)}>×</button>
				</div>
			{/each}
			{#if negativeMin}<p class="text-danger text-[13px]">{tr.products.minStockNegative}</p>{/if}
			<div class="flex items-center gap-3">
				<button type="button" class="btn btn-sm" onclick={() => drafts.push({ name: '', sku: '', min_stock: 0 })}>
					{tr.products.addVariant}
				</button>
				{#if drafts.length === 0}<span class="text-muted text-[13px]">{tr.products.standardHint}</span>{/if}
			</div>
		</fieldset>
	{/if}

	{#if conflict}
		<div class="conflict" role="alert">
			<span>{tr.common.conflictHint}</span>
			<button type="button" class="btn btn-sm" onclick={() => onRefresh?.()}>{tr.common.refresh}</button>
		</div>
	{/if}

	<div class="flex justify-end gap-2 pt-1">
		<button type="button" class="btn" onclick={onCancel}>{tr.common.cancel}</button>
		<button type="submit" class="btn btn-primary" disabled={!valid || busy}>{tr.common.save}</button>
	</div>
</form>

<style>
	.variant {
		display: grid;
		grid-template-columns: 1.4fr 1fr 110px auto;
		gap: 8px;
		align-items: end;
	}
	.conflict {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 10px;
		border-radius: 6px;
		background: var(--accent-soft);
		color: var(--accent-ink);
	}
</style>
