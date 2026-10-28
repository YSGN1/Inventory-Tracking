<script lang="ts">
	import { api } from '$lib/api';
	import type { Role } from '$lib/bindings/Role';
	import type { User } from '$lib/bindings/User';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, bumpRefresh, session, toast, ui } from '$lib/state.svelte';

	let users = $state<User[]>([]);
	let creating = $state<{ username: string; display_name: string; password: string; role: Role } | null>(null);
	let resetting = $state<User | null>(null);
	let newPassword = $state('');

	$effect(() => {
		void ui.refreshKey;
		attempt(api.listUsers()).then((r) => r && (users = r));
	});

	const roleLabel = (r: Role) => (r === 'admin' ? tr.users.admin : tr.users.operator);

	async function create(e: SubmitEvent) {
		e.preventDefault();
		if (!creating) return;
		if (await attempt(api.createUser({ ...creating, username: creating.username.trim(), display_name: creating.display_name.trim() }))) {
			toast(tr.users.created);
			creating = null;
			bumpRefresh();
		}
	}

	async function change<T>(p: Promise<T>) {
		if (await attempt(p)) {
			toast(tr.products.saved);
			bumpRefresh();
		}
	}

	async function reset(e: SubmitEvent) {
		e.preventDefault();
		if (!resetting) return;
		if (await attempt(api.setPassword(resetting.id, newPassword, resetting.row_version))) {
			toast(tr.products.saved);
			resetting = null;
			newPassword = '';
			bumpRefresh();
		}
	}
</script>

<header class="flex items-center gap-3 mb-5">
	<h1 class="text-[22px] font-semibold mr-auto">{tr.users.title}</h1>
	<button class="btn btn-primary" onclick={() => (creating = { username: '', display_name: '', password: '', role: 'operator' })}>
		{tr.users.new}
	</button>
</header>

<DataTable headers={[tr.auth.displayName, tr.auth.username, tr.users.role, '', '']}>
	{#each users as u (u.id)}
		<tr class:opacity-55={!u.active}>
			<td class="font-medium">{u.display_name}</td>
			<td class="text-muted">{u.username}</td>
			<td>
				<select
					class="input w-36"
					value={u.role}
					aria-label={tr.users.role}
					onchange={(e) => change(api.setUserRole(u.id, e.currentTarget.value as Role, u.row_version))}
				>
					<option value="admin">{roleLabel('admin')}</option>
					<option value="operator">{roleLabel('operator')}</option>
				</select>
			</td>
			<td>{u.active ? tr.users.active : tr.users.inactive}</td>
			<td class="text-right whitespace-nowrap">
				<button class="btn btn-sm btn-ghost" onclick={() => (resetting = u)}>{tr.users.resetPassword}</button>
				{#if u.id !== session.status?.user?.id}
					<button class="btn btn-sm btn-ghost" onclick={() => change(api.setUserActive(u.id, !u.active, u.row_version))}>
						{u.active ? tr.users.deactivate : tr.users.activate}
					</button>
				{/if}
			</td>
		</tr>
	{/each}
</DataTable>

<Modal open={!!creating} title={tr.users.new} onClose={() => (creating = null)}>
	{#if creating}
		<form class="grid gap-3" onsubmit={create}>
			<label class="field"><span>{tr.auth.displayName}</span><input class="input" bind:value={creating.display_name} /></label>
			<label class="field"><span>{tr.auth.username}</span><input class="input" bind:value={creating.username} autocapitalize="off" /></label>
			<label class="field"><span>{tr.auth.password}</span><input class="input" type="password" bind:value={creating.password} autocomplete="new-password" /></label>
			<label class="field">
				<span>{tr.users.role}</span>
				<select class="input" bind:value={creating.role}>
					<option value="operator">{roleLabel('operator')}</option>
					<option value="admin">{roleLabel('admin')}</option>
				</select>
			</label>
			<div class="flex justify-end gap-2">
				<button type="button" class="btn" onclick={() => (creating = null)}>{tr.common.cancel}</button>
				<button class="btn btn-primary" disabled={creating.username.trim().length < 3 || !creating.display_name.trim() || creating.password.length < 6}>
					{tr.common.add}
				</button>
			</div>
		</form>
	{/if}
</Modal>

<Modal open={!!resetting} title={`${resetting?.display_name ?? ''} — ${tr.users.resetPassword}`} onClose={() => (resetting = null)}>
	<form class="grid gap-3" onsubmit={reset}>
		<label class="field"><span>{tr.users.newPassword}</span><input class="input" type="password" bind:value={newPassword} autocomplete="new-password" /></label>
		<div class="flex justify-end gap-2">
			<button type="button" class="btn" onclick={() => (resetting = null)}>{tr.common.cancel}</button>
			<button class="btn btn-primary" disabled={newPassword.length < 6}>{tr.common.save}</button>
		</div>
	</form>
</Modal>
