<script lang="ts">
	import { dismiss, toasts } from '$lib/state.svelte';
</script>

<div class="stack" role="status" aria-live="polite">
	{#each toasts.items as t (t.id)}
		<div class="toast" class:error={t.kind === 'error'}>
			<span>{t.message}</span>
			{#if t.action}
				{@const action = t.action}
				<button
					class="act"
					onclick={async () => {
						dismiss(t.id);
						await action.run();
					}}>{action.label}</button
				>
			{/if}
			<button class="x" aria-label="Kapat" onclick={() => dismiss(t.id)}>×</button>
		</div>
	{/each}
</div>

<style>
	.stack {
		position: fixed;
		right: 16px;
		bottom: 16px;
		display: grid;
		gap: 8px;
		z-index: 60;
		max-width: min(420px, calc(100vw - 32px));
	}
	.toast {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 10px 10px 14px;
		background: var(--sidebar);
		color: #f3f5f8;
		border-radius: 8px;
		border-left: 4px solid var(--ok);
		box-shadow: 0 10px 30px -10px rgb(0 0 0 / 0.45);
	}
	.toast.error {
		border-left-color: var(--danger);
	}
	.toast span {
		flex: 1;
	}
	.act {
		font-weight: 600;
		color: var(--accent);
		padding: 4px 6px;
	}
	.x {
		color: #9aa4b2;
		font-size: 18px;
		line-height: 1;
		padding: 2px 6px;
	}
</style>
