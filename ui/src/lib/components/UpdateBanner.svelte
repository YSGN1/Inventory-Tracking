<script lang="ts">
	import { api } from '$lib/api';
	import type { UpdateInfo } from '$lib/bindings/UpdateInfo';
	import { tr } from '$lib/i18n/tr';
	import { attempt } from '$lib/state.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';

	let { info }: { info: UpdateInfo | null } = $props();
	let confirming = $state(false);
</script>

{#if info?.available}
	<div class="banner">
		<span>{tr.update.available(info.available)}</span>
		<button class="btn btn-sm" onclick={() => (confirming = true)}>{tr.update.update}</button>
	</div>
	<ConfirmDialog
		open={confirming}
		title={tr.update.available(info.available)}
		description={tr.update.confirmDesc}
		confirmLabel={tr.update.confirm}
		onCancel={() => (confirming = false)}
		onConfirm={async () => {
			// Başarılı olursa uygulama kapanıp yeni sürümle açılır.
			await attempt(api.installUpdate());
			confirming = false;
		}}
	>
		{#if info.notes}<p class="notes">{info.notes}</p>{/if}
	</ConfirmDialog>
{/if}

<style>
	.banner {
		display: grid;
		gap: 6px;
		padding: 10px;
		border-radius: 6px;
		border: 1px solid var(--accent);
		color: #f3f5f8;
		font-size: 13px;
	}
	.notes {
		white-space: pre-line;
		max-height: 240px;
		overflow: auto;
	}
</style>
