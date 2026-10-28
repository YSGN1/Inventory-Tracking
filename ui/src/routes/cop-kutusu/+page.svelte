<script lang="ts">
	import { api } from '$lib/api';
	import type { TrashItem } from '$lib/bindings/TrashItem';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import TrashTable from '$lib/components/TrashTable.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, ui } from '$lib/state.svelte';

	let items = $state<TrashItem[] | null>(null);
	let retention = $state(30);

	$effect(() => {
		void ui.refreshKey;
		attempt(api.listTrash()).then((r) => r && (items = r));
		attempt(api.getSettings()).then((s) => s && (retention = s.trash_retention_days));
	});
</script>

<header class="mb-5">
	<h1 class="text-[22px] font-semibold">{tr.trash.title}</h1>
	<p class="text-muted">{tr.trash.autoPurge(retention)}</p>
</header>

{#if items === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if items.length === 0}
	<EmptyState message={tr.trash.empty} />
{:else}
	<TrashTable {items} />
{/if}
