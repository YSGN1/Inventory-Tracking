<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import type { ProductWithVariants } from '$lib/bindings/ProductWithVariants';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ProductForm, { type ProductFormData } from '$lib/components/ProductForm.svelte';
	import ProductGrid from '$lib/components/ProductGrid.svelte';
	import { debounce, loadPref, savePref } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, isAdmin, toast, ui } from '$lib/state.svelte';

	let products = $state<ProductWithVariants[] | null>(null);
	let images = $state<Record<string, string | null>>({});
	let search = $state('');
	let query = $state('');
	let view = $state<'grid' | 'list'>(loadPref('products.view', 'list') === 'grid' ? 'grid' : 'list');
	let creating = $state(false);

	const setQuery = debounce((v: string) => (query = v), 300);

	$effect(() => {
		void ui.refreshKey;
		const q = query;
		attempt(api.listProducts(q)).then((r) => {
			if (r) {
				products = r;
				loadImages(r);
			}
		});
	});

	async function loadImages(list: ProductWithVariants[]) {
		for (const p of list) {
			if (p.product.image_path && !(p.product.id in images)) {
				images[p.product.id] = null;
				api.getProductImage(p.product.id).then((d) => (images[p.product.id] = d)).catch(() => {});
			}
		}
	}

	function setView(v: 'grid' | 'list') {
		view = v;
		savePref('products.view', v);
	}

	async function create(data: ProductFormData) {
		const p = await api.createProduct({ name: data.name, sku: data.sku || null, description: data.description || null, image_path: null, variants: data.variants });
		creating = false;
		toast(tr.products.created);
		goto(`/urunler/${p.product.id}`);
	}
</script>

<header class="head">
	<h1>{tr.products.title}</h1>
	<input
		class="input search"
		type="search"
		placeholder={tr.products.searchPlaceholder}
		aria-label={tr.common.search}
		data-search
		bind:value={search}
		oninput={() => setQuery(search)}
	/>
	<div class="seg" role="group" aria-label="Görünüm">
		<button class:on={view === 'list'} aria-pressed={view === 'list'} onclick={() => setView('list')}>{tr.products.viewList}</button>
		<button class:on={view === 'grid'} aria-pressed={view === 'grid'} onclick={() => setView('grid')}>{tr.products.viewGrid}</button>
	</div>
	{#if isAdmin()}
		<button class="btn btn-primary" onclick={() => (creating = true)}>{tr.products.new}</button>
	{/if}
</header>

{#if products === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if products.length === 0}
	<EmptyState message={query ? tr.products.noMatch : tr.products.empty}>
		{#if !query && isAdmin()}
			<button class="btn btn-primary" onclick={() => (creating = true)}>{tr.products.new}</button>
		{/if}
	</EmptyState>
{:else}
	<ProductGrid {products} {images} {view} onOpen={(id) => goto(`/urunler/${id}`)} />
{/if}

<Modal open={creating} title={tr.products.new} width={620} onClose={() => (creating = false)}>
	<ProductForm onSave={create} onCancel={() => (creating = false)} />
</Modal>

<style>
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 18px;
		flex-wrap: wrap;
	}
	h1 {
		font-size: 22px;
		font-weight: 600;
		margin-right: auto;
	}
	.search {
		width: 280px;
	}
	.seg {
		display: inline-flex;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		overflow: hidden;
	}
	.seg button {
		padding: 6px 12px;
		background: var(--surface);
	}
	.seg button.on {
		background: var(--ink);
		color: var(--paper);
	}
</style>
