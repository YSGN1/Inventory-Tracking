<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tr } from '$lib/i18n/tr';
	import Modal from './Modal.svelte';

	let {
		open,
		title,
		description = '',
		confirmLabel,
		danger = false,
		canConfirm = true,
		onConfirm,
		onCancel,
		children
	}: {
		open: boolean;
		title: string;
		description?: string;
		confirmLabel: string;
		danger?: boolean;
		canConfirm?: boolean;
		onConfirm: () => void | Promise<void>;
		onCancel: () => void;
		children?: Snippet;
	} = $props();

	let busy = $state(false);

	async function confirm() {
		if (busy || !canConfirm) return;
		busy = true;
		try {
			await onConfirm();
		} finally {
			busy = false;
		}
	}
</script>

<Modal {open} {title} onClose={onCancel}>
	{#if description}<p class="desc">{description}</p>{/if}
	{@render children?.()}
	{#snippet footer()}
		<button type="button" class="btn" onclick={onCancel}>{tr.common.cancel}</button>
		<button
			type="button"
			class="btn"
			class:btn-danger={danger}
			class:btn-primary={!danger}
			disabled={busy || !canConfirm}
			onclick={confirm}>{confirmLabel}</button
		>
	{/snippet}
</Modal>

<style>
	.desc {
		color: var(--muted);
	}
</style>
