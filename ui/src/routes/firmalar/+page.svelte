<script lang="ts">
	import { api, toApiError } from '$lib/api';
	import { trashFirmWithUndo } from '$lib/actions';
	import type { Firm } from '$lib/bindings/Firm';
	import DataTable from '$lib/components/DataTable.svelte';
	import DeleteConfirmDialog from '$lib/components/DeleteConfirmDialog.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import HistoryPanel from '$lib/components/HistoryPanel.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, bumpRefresh, isAdmin, isConflict, toast, ui } from '$lib/state.svelte';

	let firms = $state<Firm[] | null>(null);
	let editing = $state<{ id: string | null; name: string; contact: string; phone: string; note: string; row_version: number } | null>(null);
	let deleting = $state<Firm | null>(null);
	let history = $state<Firm | null>(null);
	let conflict = $state(false);
	let busy = $state(false);

	$effect(() => {
		void ui.refreshKey;
		attempt(api.listFirms()).then((r) => r && (firms = r));
	});

	function edit(f: Firm | null) {
		conflict = false;
		editing = f
			? { id: f.id, name: f.name, contact: f.contact ?? '', phone: f.phone ?? '', note: f.note ?? '', row_version: f.row_version }
			: { id: null, name: '', contact: '', phone: '', note: '', row_version: 0 };
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!editing || !editing.name.trim() || busy) return;
		busy = true;
		const data = { name: editing.name.trim(), contact: editing.contact.trim(), phone: editing.phone.trim(), note: editing.note.trim() };
		try {
			if (editing.id) await api.updateFirm(editing.id, data, editing.row_version);
			else await api.createFirm({ ...data, contact: data.contact || null, phone: data.phone || null, note: data.note || null });
			toast(editing.id ? tr.products.saved : tr.firms.created);
			editing = null;
			bumpRefresh();
		} catch (err) {
			if (isConflict(err)) conflict = true;
			else toast(toApiError(err).message, 'error');
		} finally {
			busy = false;
		}
	}
</script>

<header class="head">
	<h1>{tr.firms.title}</h1>
	{#if isAdmin()}<button class="btn btn-primary" onclick={() => edit(null)}>{tr.firms.new}</button>{/if}
</header>

{#if firms === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if firms.length === 0}
	<EmptyState message={tr.firms.empty}>
		{#if isAdmin()}<button class="btn btn-primary" onclick={() => edit(null)}>{tr.firms.new}</button>{/if}
	</EmptyState>
{:else}
	<DataTable headers={[tr.common.name, tr.firms.contact, tr.firms.phone, tr.common.note, '']}>
		{#each firms as f (f.id)}
			<tr>
				<td class="font-medium">{f.name}</td>
				<td>{f.contact ?? ''}</td>
				<td class="whitespace-nowrap">{f.phone ?? ''}</td>
				<td class="text-muted">{f.note ?? ''}</td>
				<td class="text-right whitespace-nowrap">
					<button class="btn btn-sm btn-ghost" onclick={() => (history = f)}>{tr.common.history}</button>
					{#if isAdmin()}
						<button class="btn btn-sm btn-ghost" onclick={() => edit(f)}>{tr.common.edit}</button>
						<button class="btn btn-sm btn-ghost text-danger" onclick={() => (deleting = f)}>{tr.common.delete}</button>
					{/if}
				</td>
			</tr>
		{/each}
	</DataTable>
{/if}

<Modal open={!!editing} title={editing?.id ? tr.common.edit : tr.firms.new} onClose={() => (editing = null)}>
	{#if editing}
		<form class="grid gap-3" onsubmit={save}>
			<label class="field"><span>{tr.common.name}</span><input class="input" bind:value={editing.name} maxlength="200" /></label>
			<div class="grid grid-cols-2 gap-3">
				<label class="field"><span>{tr.firms.contact}</span><input class="input" bind:value={editing.contact} maxlength="200" /></label>
				<label class="field"><span>{tr.firms.phone}</span><input class="input" bind:value={editing.phone} maxlength="50" /></label>
			</div>
			<label class="field"><span>{tr.common.note}</span><textarea class="input" rows="2" bind:value={editing.note} maxlength="1000"></textarea></label>
			{#if conflict}<p class="bg-accent-soft text-accent-ink rounded-md px-3 py-2" role="alert">{tr.common.conflictHint}</p>{/if}
			<div class="flex justify-end gap-2">
				<button type="button" class="btn" onclick={() => (editing = null)}>{tr.common.cancel}</button>
				<button class="btn btn-primary" disabled={!editing.name.trim() || busy}>{tr.common.save}</button>
			</div>
		</form>
	{/if}
</Modal>

<Modal open={!!history} title={`${history?.name ?? ''} — ${tr.common.history}`} width={720} onClose={() => (history = null)}>
	{#if history}<HistoryPanel entity="firm" id={history.id} />{/if}
</Modal>

<DeleteConfirmDialog
	open={!!deleting}
	title={tr.firms.deleteFirm(deleting?.name ?? '')}
	description={tr.firms.deleteFirmDesc}
	onCancel={() => (deleting = null)}
	onConfirm={async (confirm) => {
		const f = deleting;
		if (!f) return;
		try {
			await trashFirmWithUndo(f, confirm);
			deleting = null;
		} catch (err) {
			toast(toApiError(err).message, 'error');
		}
	}}
/>

<style>
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 18px;
	}
	h1 {
		font-size: 22px;
		font-weight: 600;
		margin-right: auto;
	}
</style>
