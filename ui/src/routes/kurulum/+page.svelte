<script lang="ts">
	import { api } from '$lib/api';
	import AuthFrame from '$lib/components/AuthFrame.svelte';
	import { tr } from '$lib/i18n/tr';
	import { attempt, session } from '$lib/state.svelte';

	let username = $state('');
	let displayName = $state('');
	let password = $state('');
	let repeat = $state('');
	let busy = $state(false);

	const mismatch = $derived(repeat.length > 0 && password !== repeat);
	const tooShort = $derived(password.length > 0 && password.length < 6);
	const valid = $derived(
		username.trim().length >= 3 && displayName.trim().length > 0 && password.length >= 6 && password === repeat
	);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!valid || busy) return;
		busy = true;
		const st = await attempt(api.setupFirstAdmin(username.trim(), displayName.trim(), password));
		busy = false;
		if (st) session.status = st;
	}
</script>

<AuthFrame title={tr.auth.setupTitle} intro={tr.auth.setupIntro}>
	<form class="grid gap-3" onsubmit={submit}>
		<label class="field">
			<span>{tr.auth.displayName}</span>
			<input class="input" bind:value={displayName} autocomplete="name" />
		</label>
		<label class="field">
			<span>{tr.auth.username}</span>
			<input class="input" bind:value={username} autocomplete="username" autocapitalize="off" />
		</label>
		<label class="field">
			<span>{tr.auth.password}</span>
			<input class="input" type="password" bind:value={password} autocomplete="new-password" aria-invalid={tooShort} />
		</label>
		{#if tooShort}<p class="text-danger text-[13px]">{tr.auth.passwordShort}</p>{/if}
		<label class="field">
			<span>{tr.auth.passwordRepeat}</span>
			<input class="input" type="password" bind:value={repeat} autocomplete="new-password" aria-invalid={mismatch} />
		</label>
		{#if mismatch}<p class="text-danger text-[13px]">{tr.auth.passwordMismatch}</p>{/if}
		<button class="btn btn-primary justify-center" disabled={!valid || busy}>{tr.auth.setupSubmit}</button>
	</form>
</AuthFrame>
