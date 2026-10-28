<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ui, openMovement } from '$lib/state.svelte';
	import Sidebar from './Sidebar.svelte';

	let { onLogout, children, overlay }: { onLogout: () => void; children: Snippet; overlay?: Snippet } = $props();

	function onKey(e: KeyboardEvent) {
		const mod = e.ctrlKey || e.metaKey;
		if (!mod) return;
		const k = e.key.toLowerCase();
		if (k === 'n') {
			e.preventDefault();
			if (!ui.movementOpen) openMovement();
		} else if (k === 'f') {
			const search = document.querySelector<HTMLInputElement>('[data-search]');
			if (search) {
				e.preventDefault();
				search.focus();
				search.select();
			}
		}
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="shell">
	<Sidebar {onLogout} />
	<main>
		{@render children()}
	</main>
</div>
{@render overlay?.()}

<style>
	.shell {
		display: grid;
		grid-template-columns: 216px minmax(0, 1fr);
		min-height: 100vh;
	}
	main {
		padding: 24px 28px 48px;
		min-width: 0;
	}
</style>
