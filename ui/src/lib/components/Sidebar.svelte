<script lang="ts">
	import { page } from '$app/state';
	import { tr } from '$lib/i18n/tr';
	import { isAdmin, openMovement, session } from '$lib/state.svelte';

	let { onLogout }: { onLogout: () => void } = $props();

	const links = $derived([
		{ href: '/', label: tr.nav.summary },
		{ href: '/urunler', label: tr.nav.products },
		{ href: '/hareketler', label: tr.nav.movements },
		{ href: '/firmalar', label: tr.nav.firms },
		{ href: '/raporlar', label: tr.nav.reports },
		...(isAdmin()
			? [
					{ href: '/cop-kutusu', label: tr.nav.trash },
					{ href: '/kullanicilar', label: tr.nav.users }
				]
			: []),
		{ href: '/ayarlar', label: tr.nav.settings }
	]);

	const active = (href: string) =>
		href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);

	const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);

	let theme = $state(typeof document !== 'undefined' ? document.documentElement.dataset.theme : 'light');
	function toggleTheme() {
		theme = theme === 'dark' ? 'light' : 'dark';
		document.documentElement.dataset.theme = theme;
		try {
			localStorage.setItem('theme', theme ?? 'light');
		} catch {
			/* tercih saklanamazsa yalnız bu oturumda geçerli */
		}
	}
</script>

<aside>
	<div class="brand">
		<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
			<rect x="8" y="3" width="8" height="8" fill="var(--accent)" />
			<rect x="3" y="13" width="8" height="8" fill="currentColor" />
			<rect x="13" y="13" width="8" height="8" fill="currentColor" />
		</svg>
		<span>{tr.app.name}</span>
	</div>

	<button class="new" onclick={() => openMovement()}>
		<span>{tr.nav.newMovement}</span>
		<kbd>{isMac ? '⌘' : 'Ctrl'}+N</kbd>
	</button>

	<nav>
		{#each links as l (l.href)}
			<a href={l.href} class:active={active(l.href)} aria-current={active(l.href) ? 'page' : undefined}>{l.label}</a>
		{/each}
	</nav>

	<div class="foot">
		<div class="who">
			<strong>{session.status?.user?.display_name}</strong>
			<span>{session.status?.user?.role === 'admin' ? tr.users.admin : tr.users.operator}</span>
		</div>
		<div class="row">
			<button class="link" onclick={toggleTheme}>{theme === 'dark' ? tr.nav.themeLight : tr.nav.themeDark} {tr.nav.theme.toLowerCase()}</button>
			{#if !session.status?.single_user_mode}
				<button class="link" onclick={onLogout}>{tr.nav.logout}</button>
			{/if}
		</div>
	</div>
</aside>

<style>
	aside {
		background: var(--sidebar);
		color: var(--sidebar-ink);
		display: flex;
		flex-direction: column;
		gap: 18px;
		padding: 18px 12px;
		height: 100vh;
		position: sticky;
		top: 0;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		color: #f3f5f8;
		font-weight: 600;
		font-size: 15px;
		padding: 0 8px;
	}
	.new {
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: var(--accent);
		color: #1b2330;
		font-weight: 600;
		border-radius: 6px;
		padding: 8px 10px;
	}
	kbd {
		font: inherit;
		font-size: 11.5px;
		font-weight: 500;
		opacity: 0.7;
	}
	nav {
		display: grid;
		gap: 2px;
	}
	nav a {
		padding: 7px 10px;
		border-radius: 6px;
		color: inherit;
		text-decoration: none;
	}
	nav a:hover {
		color: #fff;
	}
	nav a.active {
		background: var(--sidebar-active);
		color: #fff;
		box-shadow: inset 3px 0 0 var(--accent);
	}
	.foot {
		margin-top: auto;
		display: grid;
		gap: 8px;
		padding: 10px 8px 0;
		border-top: 1px solid rgb(255 255 255 / 0.08);
	}
	.who {
		display: grid;
	}
	.who strong {
		color: #f3f5f8;
		font-weight: 500;
	}
	.who span {
		font-size: 12px;
	}
	.row {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}
	.link {
		font-size: 12.5px;
		color: inherit;
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.link:hover {
		color: #fff;
	}
</style>
