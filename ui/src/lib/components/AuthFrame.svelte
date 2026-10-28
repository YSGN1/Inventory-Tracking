<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tr } from '$lib/i18n/tr';

	let { title, intro = '', children }: { title: string; intro?: string; children: Snippet } = $props();
</script>

<div class="frame">
	<div class="mark" aria-hidden="true">
		<span class="a"></span><span class="b"></span><span class="c"></span>
	</div>
	<section>
		<p class="app">{tr.app.name}</p>
		<h1>{title}</h1>
		{#if intro}<p class="intro">{intro}</p>{/if}
		{@render children()}
	</section>
</div>

<style>
	.frame {
		min-height: 100vh;
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(320px, 420px) minmax(0, 1.4fr);
		align-items: center;
		gap: 48px;
		padding: 32px;
	}
	/* Logodaki üç kutu, büyük ölçekte: istif fikri */
	.mark {
		justify-self: end;
		display: grid;
		grid-template-columns: repeat(2, 64px);
		grid-template-rows: repeat(2, 64px);
		gap: 10px;
	}
	.mark span {
		display: block;
		background: var(--ink);
		border-radius: 4px;
	}
	.mark .a {
		grid-column: 1 / span 2;
		width: 64px;
		justify-self: center;
		background: var(--accent);
	}
	section {
		grid-column: 2;
		display: grid;
		gap: 14px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 10px;
		padding: 28px;
	}
	.app {
		color: var(--muted);
		font-weight: 500;
	}
	h1 {
		font-size: 24px;
		font-weight: 600;
		line-height: 1.2;
		letter-spacing: -0.01em;
	}
	.intro {
		color: var(--muted);
	}
	@media (max-width: 820px) {
		.frame {
			grid-template-columns: 1fr;
		}
		.mark {
			display: none;
		}
		section {
			grid-column: 1;
		}
	}
</style>
