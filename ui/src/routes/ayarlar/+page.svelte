<script lang="ts">
	import { open as openFile, save } from '@tauri-apps/plugin-dialog';
	import { api } from '$lib/api';
	import type { Settings } from '$lib/bindings/Settings';
	import DeleteConfirmDialog from '$lib/components/DeleteConfirmDialog.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, isAdmin, session, toast } from '$lib/state.svelte';

	let settings = $state<Settings | null>(null);
	let retention = $state(30);
	let restoreFrom = $state<string | null>(null);
	let pwOpen = $state(false);
	let newPassword = $state('');

	async function load() {
		const s = await attempt(api.getSettings());
		if (s) {
			settings = s;
			retention = s.trash_retention_days;
		}
	}
	$effect(() => {
		load();
	});

	const retentionValid = $derived(Number.isInteger(retention) && retention >= 1 && retention <= 365);

	async function setSingle(on: boolean) {
		if ((await attempt(api.setSingleUserMode(on))) !== undefined) {
			toast(tr.settings.saved);
			if (session.status) session.status = { ...session.status, single_user_mode: on };
			load();
		}
	}

	async function saveRetention() {
		if (!retentionValid) return;
		if ((await attempt(api.setTrashRetentionDays(retention))) !== undefined) toast(tr.settings.saved);
	}

	async function backup() {
		const stamp = new Date().toISOString().slice(0, 16).replace(/[-:T]/g, '');
		const dest = await save({ defaultPath: `stok-yedek-${stamp}.db`, filters: [{ name: 'Stok yedeği', extensions: ['db'] }] });
		if (!dest) return;
		if ((await attempt(api.backupNow(dest))) !== undefined) toast(tr.settings.backedUp);
	}

	async function chooseRestore() {
		const src = await openFile({ multiple: false, filters: [{ name: 'Stok yedeği', extensions: ['db'] }] });
		if (typeof src === 'string') restoreFrom = src;
	}

	async function changePassword(e: SubmitEvent) {
		e.preventDefault();
		const u = session.status?.user;
		if (!u) return;
		if (await attempt(api.setPassword(u.id, newPassword, u.row_version))) {
			toast(tr.settings.saved);
			pwOpen = false;
			newPassword = '';
			const st = await attempt(api.appStatus());
			if (st) session.status = st;
		}
	}
</script>

<h1 class="text-[22px] font-semibold mb-5">{tr.settings.title}</h1>

<div class="sections">
	{#if isAdmin() && settings}
		<section>
			<label class="row">
				<input type="checkbox" checked={settings.single_user_mode} onchange={(e) => setSingle(e.currentTarget.checked)} />
				<div>
					<strong>{tr.settings.singleUser}</strong>
					<p class="text-muted">{tr.settings.singleUserHint}</p>
				</div>
			</label>
		</section>

		<section>
			<form
				class="flex items-end gap-3"
				onsubmit={(e) => {
					e.preventDefault();
					saveRetention();
				}}
			>
				<label class="field">
					<span>{tr.settings.retention}</span>
					<input class="input num w-28" type="number" min="1" max="365" step="1" bind:value={retention} aria-invalid={!retentionValid} />
				</label>
				<button class="btn" disabled={!retentionValid}>{tr.common.save}</button>
			</form>
			<p class="text-muted mt-1">{tr.settings.retentionHint}</p>
		</section>

		<section>
			<strong>{tr.settings.backups}</strong>
			<p class="text-muted">{tr.settings.autoBackupHint}</p>
			<div class="flex gap-2 mt-3 flex-wrap">
				<button class="btn" onclick={backup}>{tr.settings.backupNow}</button>
				<button class="btn" onclick={chooseRestore}>{tr.settings.restore}</button>
				<button class="btn btn-ghost" onclick={() => attempt(api.openBackupsDir())}>{tr.settings.openBackups}</button>
			</div>
		</section>
	{/if}

	<section>
		<button class="btn" onclick={() => (pwOpen = true)}>{tr.settings.changePassword}</button>
	</section>
</div>

<DeleteConfirmDialog
	open={!!restoreFrom}
	title={tr.settings.restoreTitle}
	description={`${tr.settings.restoreDesc}\n${restoreFrom ?? ''}`}
	confirmLabel={tr.settings.restore}
	onCancel={() => (restoreFrom = null)}
	onConfirm={async () => {
		const src = restoreFrom;
		if (!src) return;
		const st = await attempt(api.restoreBackup(src));
		restoreFrom = null;
		if (st) {
			toast(tr.settings.restored);
			session.status = st;
		}
	}}
/>

<Modal open={pwOpen} title={tr.settings.changePassword} onClose={() => (pwOpen = false)}>
	<form class="grid gap-3" onsubmit={changePassword}>
		<label class="field"><span>{tr.users.newPassword}</span><input class="input" type="password" bind:value={newPassword} autocomplete="new-password" /></label>
		<div class="flex justify-end gap-2">
			<button type="button" class="btn" onclick={() => (pwOpen = false)}>{tr.common.cancel}</button>
			<button class="btn btn-primary" disabled={newPassword.length < 6}>{tr.common.save}</button>
		</div>
	</form>
</Modal>

<style>
	.sections {
		display: grid;
		gap: 0;
		max-width: 640px;
		border: 1px solid var(--line);
		border-radius: 8px;
		background: var(--surface);
	}
	section {
		padding: 16px 18px;
		border-bottom: 1px solid var(--line);
	}
	section:last-child {
		border-bottom: 0;
	}
	.row {
		display: flex;
		gap: 12px;
		align-items: flex-start;
	}
	.row input {
		margin-top: 4px;
	}
	strong {
		font-weight: 500;
	}
</style>
