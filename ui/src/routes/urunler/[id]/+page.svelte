<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { open as openFile } from '@tauri-apps/plugin-dialog';
	import { api, toApiError } from '$lib/api';
	import { trashProductWithUndo } from '$lib/actions';
	import type { ProductWithVariants } from '$lib/bindings/ProductWithVariants';
	import DeleteConfirmDialog from '$lib/components/DeleteConfirmDialog.svelte';
	import HistoryPanel from '$lib/components/HistoryPanel.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ProductForm, { type ProductFormData } from '$lib/components/ProductForm.svelte';
	import VariantTable from '$lib/components/VariantTable.svelte';
	import { fmtNum, initial } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, bumpRefresh, isAdmin, toast, ui } from '$lib/state.svelte';

	const id = $derived(page.params.id ?? '');
	let data = $state<ProductWithVariants | null>(null);
	let image = $state<string | null>(null);
	let tab = $state<'variants' | 'history'>('variants');
	let editing = $state(false);
	let deleting = $state(false);
	let formKey = $state(0);

	async function load() {
		const r = await attempt(api.getProduct(id));
		if (!r) {
			goto('/urunler');
			return;
		}
		data = r;
		image = r.product.image_path ? await api.getProductImage(id).catch(() => null) : null;
	}

	$effect(() => {
		void ui.refreshKey;
		void id;
		load();
	});

	async function save(f: ProductFormData) {
		if (!data) return;
		await api.updateProduct(id, { name: f.name, sku: f.sku, description: f.description }, data.product.row_version);
		editing = false;
		toast(tr.products.saved);
		bumpRefresh();
	}

	async function chooseImage() {
		const path = await openFile({ multiple: false, filters: [{ name: tr.products.image, extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
		if (typeof path !== 'string') return;
		if (await attempt(api.setProductImage(id, path))) bumpRefresh();
	}

	async function removeImage() {
		if (!data) return;
		if (await attempt(api.updateProduct(id, { image_path: '' }, data.product.row_version))) bumpRefresh();
	}
</script>

{#if data}
	<a class="back" href="/urunler">{tr.products.title}</a>
	<header class="head">
		<div class="pic">
			{#if image}
				<img src={image} alt={data.product.name} />
			{:else}
				<span aria-hidden="true">{initial(data.product.name)}</span>
			{/if}
		</div>
		<div class="title">
			<h1>{data.product.name}</h1>
			<p class="text-muted">
				{#if data.product.sku}{data.product.sku}{/if}
			</p>
			{#if data.product.description}<p class="desc">{data.product.description}</p>{/if}
			{#if isAdmin()}
				<div class="flex gap-2 mt-2 flex-wrap">
					<button class="btn btn-sm" onclick={() => { formKey++; editing = true; }}>{tr.common.edit}</button>
					<button class="btn btn-sm" onclick={chooseImage}>{tr.products.chooseImage}</button>
					{#if image}<button class="btn btn-sm btn-ghost" onclick={removeImage}>{tr.products.removeImage}</button>{/if}
					<button class="btn btn-sm btn-ghost text-danger" onclick={() => (deleting = true)}>{tr.common.delete}</button>
				</div>
			{/if}
		</div>
		<div class="total">
			<span class="text-muted">{tr.products.totalStock}</span>
			<strong class="num">{fmtNum(data.total_stock)}</strong>
		</div>
	</header>

	<div class="tabs" role="tablist">
		<button role="tab" aria-selected={tab === 'variants'} class:on={tab === 'variants'} onclick={() => (tab = 'variants')}>{tr.products.variants}</button>
		<button role="tab" aria-selected={tab === 'history'} class:on={tab === 'history'} onclick={() => (tab = 'history')}>{tr.common.history}</button>
	</div>

	{#if tab === 'variants'}
		<VariantTable productId={id} variants={data.variants} admin={isAdmin()} />
	{:else}
		<HistoryPanel entity="product" {id} />
	{/if}

	<Modal open={editing} title={tr.common.edit} width={560} onClose={() => (editing = false)}>
		{#key formKey}
			<ProductForm
				initial={data.product}
				onSave={save}
				onCancel={() => (editing = false)}
				onRefresh={async () => {
					await load();
					formKey++;
				}}
			/>
		{/key}
	</Modal>

	<DeleteConfirmDialog
		open={deleting}
		title={tr.products.deleteProduct(data.product.name)}
		description={tr.products.deleteProductDesc}
		onCancel={() => (deleting = false)}
		onConfirm={async (confirm) => {
			if (!data) return;
			try {
				await trashProductWithUndo(data.product, confirm);
				deleting = false;
				goto('/urunler');
			} catch (err) {
				toast(toApiError(err).message, 'error');
			}
		}}
	/>
{:else}
	<p class="text-muted">{tr.common.loading}</p>
{/if}

<style>
	.back {
		color: var(--muted);
		font-size: 13px;
		text-decoration: none;
	}
	.back::before {
		content: '‹ ';
	}
	.head {
		display: grid;
		grid-template-columns: 112px 1fr auto;
		gap: 20px;
		align-items: start;
		margin: 10px 0 22px;
	}
	.pic {
		width: 112px;
		height: 112px;
		border-radius: 8px;
		border: 1px solid var(--line);
		background: var(--surface);
		display: grid;
		place-items: center;
		overflow: hidden;
		font-size: 44px;
		font-weight: 600;
		color: var(--muted);
	}
	.pic img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	h1 {
		font-size: 24px;
		font-weight: 600;
		line-height: 1.2;
	}
	.desc {
		max-width: 70ch;
		margin-top: 6px;
	}
	.total {
		display: grid;
		justify-items: end;
	}
	.total strong {
		font-size: 40px;
		font-weight: 600;
		line-height: 1;
		letter-spacing: -0.02em;
	}
	.tabs {
		display: flex;
		gap: 4px;
		border-bottom: 1px solid var(--line);
		margin-bottom: 14px;
	}
	.tabs button {
		padding: 8px 12px;
		color: var(--muted);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
	}
	.tabs button.on {
		color: var(--ink);
		border-bottom-color: var(--accent);
		font-weight: 500;
	}
</style>
