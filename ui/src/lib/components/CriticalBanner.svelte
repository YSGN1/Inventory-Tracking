<script lang="ts">
	import type { CriticalItem } from '$lib/bindings/CriticalItem';
	import { tr } from '$lib/i18n/tr';
	import { openMovement } from '$lib/state.svelte';

	let { items }: { items: CriticalItem[] } = $props();
</script>

{#if items.length > 0}
	<section class="banner" aria-label={tr.summary.critical(items.length)}>
		<p class="head">{tr.summary.critical(items.length)}</p>
		<ul>
			{#each items as i (i.variant_id)}
				<li>
					<button onclick={() => openMovement(i.variant_id)}>
						<span>{i.product_name} — {i.variant_name}</span>
						<strong class="num">{tr.summary.criticalItem(i.stock_qty, i.min_stock)}</strong>
					</button>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.banner {
		border: 1px solid var(--accent);
		background: var(--accent-soft);
		color: var(--accent-ink);
		border-radius: 8px;
		padding: 12px 14px;
		margin-bottom: 18px;
	}
	.head {
		font-weight: 600;
		margin-bottom: 8px;
	}
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	button {
		display: inline-flex;
		gap: 10px;
		align-items: baseline;
		padding: 4px 10px;
		border-radius: 999px;
		background: var(--surface);
		border: 1px solid var(--accent);
		color: var(--ink);
	}
	strong {
		font-weight: 600;
		color: var(--accent-ink);
	}
</style>
