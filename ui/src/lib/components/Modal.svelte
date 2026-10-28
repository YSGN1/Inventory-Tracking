<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open,
		title,
		onClose,
		width = 440,
		children,
		footer
	}: {
		open: boolean;
		title: string;
		onClose: () => void;
		width?: number;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	let panel: HTMLDivElement | undefined = $state();

	$effect(() => {
		if (open && panel) {
			const first = panel.querySelector<HTMLElement>('[data-autofocus], input, select, textarea, button');
			first?.focus();
		}
	});

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation();
			onClose();
		}
	}
</script>

{#if open}
	<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onClose()}>
		<div
			bind:this={panel}
			class="panel"
			style:max-width="{width}px"
			role="dialog"
			aria-modal="true"
			aria-labelledby="modal-title"
			tabindex="-1"
			onkeydown={onKey}
		>
			<h2 id="modal-title">{title}</h2>
			<div class="body">{@render children()}</div>
			{#if footer}
				<div class="footer">{@render footer()}</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: color-mix(in srgb, var(--ink) 35%, transparent);
		display: grid;
		place-items: start center;
		padding: 10vh 16px 16px;
		z-index: 50;
		overflow: auto;
	}
	.panel {
		width: 100%;
		background: var(--surface);
		border: 1px solid var(--line-strong);
		border-radius: 10px;
		box-shadow: 0 18px 48px -12px color-mix(in srgb, var(--ink) 40%, transparent);
		animation: rise 140ms ease-out;
	}
	h2 {
		font-size: 16px;
		font-weight: 600;
		padding: 16px 20px 0;
	}
	.body {
		padding: 12px 20px 16px;
		display: grid;
		gap: 12px;
	}
	.footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		padding: 12px 20px;
		border-top: 1px solid var(--line);
		background: var(--surface-2);
		border-radius: 0 0 10px 10px;
	}
	@keyframes rise {
		from {
			transform: translateY(6px);
			opacity: 0;
		}
	}
</style>
