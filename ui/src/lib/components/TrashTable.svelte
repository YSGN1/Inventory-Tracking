<script module lang="ts">
	const DAY = 86_400_000;
	/** Kalıcı silinmeye kalan gün (yukarı yuvarlanır, en az 0). */
	export const daysLeft = (purgeAt: number, now: number) => Math.max(0, Math.ceil((purgeAt - now) / DAY));
</script>

<script lang="ts">
	import { api, toApiError } from '$lib/api';
	import type { TrashItem } from '$lib/bindings/TrashItem';
	import { fmtDate, fmtNum } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, bumpRefresh, toast } from '$lib/state.svelte';
	import DataTable from './DataTable.svelte';
	import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';

	let { items, now = Date.now() }: { items: TrashItem[]; now?: number } = $props();

	let purging = $state<TrashItem | null>(null);

	const fullName = (i: TrashItem) => (i.parent_name ? `${i.parent_name} / ${i.name}` : i.name);

	async function restore(i: TrashItem) {
		if ((await attempt(api.restoreItem(i.kind, i.id, i.row_version))) !== undefined) {
			toast(tr.trash.restored);
			bumpRefresh();
		}
	}

	async function purge(confirm: string, withHistory: boolean) {
		const i = purging;
		if (!i) return;
		try {
			const r = await api.purgeItem(i.kind, i.id, confirm, withHistory);
			purging = null;
			toast(r.note ?? tr.trash.purged);
			bumpRefresh();
		} catch (e) {
			toast(toApiError(e).message, 'error');
		}
	}
</script>

<DataTable
	headers={[tr.trash.kind, tr.common.name, tr.trash.deletedAt, tr.trash.purgeIn, tr.trash.stock, '']}
	numeric={[3, 4]}
>
	{#each items as i (i.kind + i.id)}
		<tr>
			<td class="text-muted">{tr.trash.kinds[i.kind]}</td>
			<td>{fullName(i)}</td>
			<td class="text-muted">{fmtDate(i.deleted_at)}</td>
			<td class="num">{tr.trash.days(daysLeft(i.purge_at, now))}</td>
			<td class="num">{i.kind === 'firm' ? '—' : fmtNum(i.stock_qty)}</td>
			<td class="actions">
				<button class="btn btn-sm" onclick={() => restore(i)}>{tr.trash.restore}</button>
				<button class="btn btn-sm btn-ghost text-danger" onclick={() => (purging = i)}>{tr.trash.purge}</button>
			</td>
		</tr>
	{/each}
</DataTable>

<DeleteConfirmDialog
	open={!!purging}
	title={tr.trash.purgeTitle(purging ? fullName(purging) : '')}
	description={tr.trash.purgeDesc}
	showHistoryOption
	onCancel={() => (purging = null)}
	onConfirm={purge}
/>

<style>
	.actions {
		text-align: right;
		white-space: nowrap;
	}
</style>
