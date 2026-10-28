<script lang="ts">
	import { api } from '$lib/api';
	import type { CriticalItem } from '$lib/bindings/CriticalItem';
	import type { StockSummaryRow } from '$lib/bindings/StockSummaryRow';
	import CriticalBanner from '$lib/components/CriticalBanner.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import StockSummaryTable from '$lib/components/StockSummaryTable.svelte';
	import { dayEnd, dayStart, debounce } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, openMovement, ui } from '$lib/state.svelte';

	let rows = $state<StockSummaryRow[] | null>(null);
	let critical = $state<CriticalItem[]>([]);
	let from = $state('');
	let to = $state('');
	let search = $state('');
	let query = $state('');
	const setQuery = debounce((v: string) => (query = v), 300);

	$effect(() => {
		void ui.refreshKey;
		const q = { search: query || null, from_ms: dayStart(from), to_ms: dayEnd(to) };
		attempt(api.stockSummary(q)).then((r) => r && (rows = r));
		attempt(api.criticalStock()).then((r) => r && (critical = r));
	});
</script>

<header class="head">
	<h1>{tr.summary.title}</h1>
	<input
		class="input w-64"
		type="search"
		placeholder={tr.products.searchPlaceholder}
		aria-label={tr.common.search}
		data-search
		bind:value={search}
		oninput={() => setQuery(search)}
	/>
	<label class="range">
		<span>{tr.common.from}</span>
		<input class="input" type="date" bind:value={from} />
	</label>
	<label class="range">
		<span>{tr.common.to}</span>
		<input class="input" type="date" bind:value={to} />
	</label>
</header>

<CriticalBanner items={critical} />

{#if rows === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if rows.length === 0}
	<EmptyState message={query ? tr.products.noMatch : tr.summary.empty}>
		{#if !query}<button class="btn btn-primary" onclick={() => openMovement()}>{tr.nav.newMovement}</button>{/if}
	</EmptyState>
{:else}
	<StockSummaryTable {rows} />
{/if}

<style>
	.head {
		display: flex;
		align-items: end;
		gap: 12px;
		margin-bottom: 18px;
		flex-wrap: wrap;
	}
	h1 {
		font-size: 22px;
		font-weight: 600;
		margin-right: auto;
		align-self: center;
	}
	.range {
		display: grid;
		gap: 2px;
		font-size: 12px;
		color: var(--muted);
	}
	.range .input {
		width: 150px;
	}
</style>
