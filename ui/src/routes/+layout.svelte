<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount, type Snippet } from 'svelte';
	import { api } from '$lib/api';
	import AppShell from '$lib/components/AppShell.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { tr } from '$lib/i18n/tr';
	import MovementDialog from '$lib/components/MovementDialog.svelte';
	import { attempt, hooks, session, ui } from '$lib/state.svelte';

	let { children }: { children: Snippet } = $props();

	const AUTH_PAGES = ['/kurulum', '/giris'];
	const onAuthPage = $derived(AUTH_PAGES.includes(page.url.pathname));

	async function refreshStatus() {
		const st = await attempt(api.appStatus());
		if (st) session.status = st;
	}

	hooks.onUnauthenticated = () => {
		if (session.status) session.status = { ...session.status, user: null };
	};

	// Durum değiştikçe doğru ekrana yönlendir.
	$effect(() => {
		const st = session.status;
		if (!st) return;
		const path = page.url.pathname;
		if (!st.initialized) {
			if (path !== '/kurulum') goto('/kurulum', { replaceState: true });
		} else if (!st.user) {
			if (path !== '/giris') goto('/giris', { replaceState: true });
		} else if (AUTH_PAGES.includes(path)) {
			goto('/', { replaceState: true });
		}
	});

	async function logout() {
		await attempt(api.logout());
		await refreshStatus();
	}

	onMount(refreshStatus);
</script>

{#if !session.status}
	<p class="boot">{tr.common.loading}</p>
{:else if session.status.user && !onAuthPage}
	<AppShell onLogout={logout}>
		{@render children()}
	</AppShell>
	<MovementDialog open={ui.movementOpen} variantId={ui.movementVariantId} onClose={() => (ui.movementOpen = false)} />
{:else if onAuthPage}
	{@render children()}
{/if}
<Toast />

<style>
	.boot {
		padding: 40px;
		color: var(--muted);
	}
</style>
