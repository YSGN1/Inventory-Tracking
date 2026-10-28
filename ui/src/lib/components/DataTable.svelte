<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		headers,
		numeric = [],
		children
	}: {
		headers: string[];
		/** Sağa hizalı (sayı) sütunların sırası. */
		numeric?: number[];
		children: Snippet;
	} = $props();
</script>

<div class="wrap">
	<table class="data">
		<thead>
			<tr>
				{#each headers as h, i (i)}
					<th class:num={numeric.includes(i)}>{h}</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			{@render children()}
		</tbody>
	</table>
</div>

<style>
	.wrap {
		border: 1px solid var(--line);
		border-radius: 8px;
		overflow: auto;
		background: var(--surface);
	}
	.wrap :global(tbody tr:last-child td) {
		border-bottom: 0;
	}
</style>
