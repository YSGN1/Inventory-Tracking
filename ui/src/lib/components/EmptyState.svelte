<script lang="ts">
	import type { Snippet } from 'svelte';

	let { message, children }: { message: string; children?: Snippet } = $props();
</script>

<div class="empty">
	<p>{message}</p>
	{@render children?.()}
</div>

<style>
	.empty {
		display: grid;
		justify-items: start;
		gap: 12px;
		padding: 28px 24px;
		border: 1px dashed var(--line-strong);
		border-radius: 8px;
		color: var(--muted);
	}
</style>
