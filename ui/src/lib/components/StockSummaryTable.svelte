<script lang="ts">
	import type { StockSummaryRow } from '$lib/bindings/StockSummaryRow';
	import { fmtNum, fmtSigned } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import DataTable from './DataTable.svelte';

	let { rows }: { rows: StockSummaryRow[] } = $props();

	const critical = (r: StockSummaryRow) => r.min_stock > 0 && r.current <= r.min_stock;
	// Eşik çubuğu: eşiğin iki katı dolu sayılır.
	const fill = (r: StockSummaryRow) => Math.min(100, Math.round((r.current / (r.min_stock * 2)) * 100));
</script>

<DataTable headers={[...tr.summary.headers]} numeric={[1, 2, 3, 4]}>
	{#each rows as r (r.variant_id)}
		<tr class:critical={critical(r)}>
			<td>
				<a href="/urunler/{r.product_id}" class="name">{r.product_name}</a>
				<span class="variant">{r.variant_name}</span>
			</td>
			<td class="num in">{fmtNum(r.total_in)}</td>
			<td class="num out">{fmtNum(r.total_out)}</td>
			<td class="num adj">{r.total_adjust === 0 ? '0' : fmtSigned(r.total_adjust)}</td>
			<td class="num cur">
				<strong>{fmtNum(r.current)}</strong>
				{#if r.min_stock > 0}
					<span class="bar" title="{tr.products.minStock}: {r.min_stock}"><i style:width="{fill(r)}%"></i></span>
				{/if}
			</td>
		</tr>
	{/each}
</DataTable>

<style>
	.name {
		color: inherit;
		text-decoration: none;
		font-weight: 500;
	}
	.name:hover {
		text-decoration: underline;
	}
	.variant {
		display: block;
		color: var(--muted);
		font-size: 12.5px;
	}
	.in {
		color: var(--ok);
	}
	.out {
		color: var(--danger);
	}
	.adj {
		color: var(--muted);
	}
	.cur strong {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.bar {
		display: block;
		margin: 4px 0 0 auto;
		width: 64px;
		height: 3px;
		background: var(--line);
		border-radius: 2px;
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
		background: var(--ok);
	}
	tr.critical {
		background: var(--accent-soft);
	}
	tr.critical .cur strong {
		color: var(--accent-ink);
	}
	tr.critical .bar i {
		background: var(--accent);
	}
</style>
