<script lang="ts">
	import type { ProductWithVariants } from '$lib/bindings/ProductWithVariants';
	import { fmtNum, initial } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import DataTable from './DataTable.svelte';

	let {
		products,
		images,
		view,
		onOpen
	}: {
		products: ProductWithVariants[];
		/** product id → data URL */
		images: Record<string, string | null | undefined>;
		view: 'grid' | 'list';
		onOpen: (id: string) => void;
	} = $props();

	const lowStock = (p: ProductWithVariants) => p.variants.some((v) => v.min_stock > 0 && v.stock_qty <= v.min_stock);
</script>

{#if view === 'grid'}
	<ul class="grid-view">
		{#each products as p (p.product.id)}
			<li>
				<button class="card" onclick={() => onOpen(p.product.id)}>
					<div class="thumb">
						{#if images[p.product.id]}
							<img src={images[p.product.id]} alt={p.product.name} />
						{:else}
							<span class="badge" aria-hidden="true">{initial(p.product.name)}</span>
						{/if}
					</div>
					<div class="meta">
						<strong>{p.product.name}</strong>
						<span class="sub">{tr.products.variantCount(p.variants.length)}</span>
					</div>
					<span class="qty num" class:low={lowStock(p)}>{fmtNum(p.total_stock)}</span>
				</button>
			</li>
		{/each}
	</ul>
{:else}
	<DataTable headers={[tr.common.name, tr.common.sku, tr.products.variants, tr.products.totalStock]} numeric={[3]}>
		{#each products as p (p.product.id)}
			<tr class="row" onclick={() => onOpen(p.product.id)}>
				<td>
					<a
						href="/urunler/{p.product.id}"
						onclick={(e) => {
							e.preventDefault();
							onOpen(p.product.id);
						}}>{p.product.name}</a
					>
				</td>
				<td class="text-muted">{p.product.sku ?? ''}</td>
				<td>{tr.products.variantCount(p.variants.length)}</td>
				<td class="num" class:low={lowStock(p)}>{fmtNum(p.total_stock)}</td>
			</tr>
		{/each}
	</DataTable>
{/if}

<style>
	.grid-view {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
		gap: 14px;
	}
	.card {
		display: grid;
		grid-template-columns: 1fr auto;
		grid-template-rows: auto auto;
		width: 100%;
		text-align: left;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 8px;
		overflow: hidden;
	}
	.card:hover {
		border-color: var(--line-strong);
	}
	.thumb {
		grid-column: 1 / -1;
		aspect-ratio: 4 / 3;
		background: var(--surface-2);
		display: grid;
		place-items: center;
		border-bottom: 1px solid var(--line);
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.badge {
		font-size: 44px;
		font-weight: 600;
		color: var(--muted);
	}
	.meta {
		display: grid;
		padding: 10px 12px;
		min-width: 0;
	}
	.meta strong {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sub {
		font-size: 12.5px;
		color: var(--muted);
	}
	.qty {
		align-self: center;
		padding: 0 12px;
		font-size: 22px;
		font-weight: 600;
	}
	.low {
		color: var(--accent-ink);
		background: var(--accent-soft);
		border-radius: 4px;
	}
	.qty.low {
		margin-right: 10px;
		padding: 0 8px;
	}
	.row {
		cursor: pointer;
	}
	.row a {
		color: inherit;
		text-decoration: none;
		font-weight: 500;
	}
</style>
