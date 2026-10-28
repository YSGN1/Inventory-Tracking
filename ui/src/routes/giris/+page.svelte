<script lang="ts">
	import { api } from '$lib/api';
	import AuthFrame from '$lib/components/AuthFrame.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, session } from '$lib/state.svelte';

	let username = $state('');
	let password = $state('');
	let busy = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!username || !password || busy) return;
		busy = true;
		const st = await attempt(api.login(username.trim(), password));
		busy = false;
		if (st) session.status = st;
		else password = '';
	}
</script>

<AuthFrame title={tr.auth.loginTitle}>
	<form class="grid gap-3" onsubmit={submit}>
		<label class="field">
			<span>{tr.auth.username}</span>
			<input class="input" bind:value={username} autocomplete="username" autocapitalize="off" />
		</label>
		<label class="field">
			<span>{tr.auth.password}</span>
			<input class="input" type="password" bind:value={password} autocomplete="current-password" />
		</label>
		<button class="btn btn-primary justify-center" disabled={!username || !password || busy}>{tr.auth.loginSubmit}</button>
	</form>
</AuthFrame>
