<script lang="ts">
	import { tr } from '$lib/i18n/tr';
	import Modal from './Modal.svelte';

	const WORD = 'SİL';

	let {
		open,
		title,
		description,
		showHistoryOption = false,
		historyLabel = tr.del.withHistory,
		confirmLabel = tr.del.button,
		onConfirm,
		onCancel
	}: {
		open: boolean;
		title: string;
		description: string;
		showHistoryOption?: boolean;
		historyLabel?: string;
		confirmLabel?: string;
		onConfirm: (confirm: string, withHistory: boolean) => void | Promise<void>;
		onCancel: () => void;
	} = $props();

	let typed = $state('');
	let withHistory = $state(false);
	let busy = $state(false);
	let copied = $state(false);

	const ready = $derived(typed.trim() === WORD);

	$effect(() => {
		if (!open) {
			typed = '';
			withHistory = false;
			copied = false;
		}
	});

	async function copyWord() {
		try {
			await navigator.clipboard.writeText(WORD);
			copied = true;
		} catch {
			// Pano erişimi yoksa kelimeyi alana yazmak da aynı işi görür.
			typed = WORD;
		}
	}

	async function confirm() {
		if (!ready || busy) return;
		busy = true;
		try {
			await onConfirm(WORD, showHistoryOption && withHistory);
		} finally {
			busy = false;
		}
	}
</script>

<Modal {open} {title} onClose={onCancel}>
	<p class="desc">{description}</p>
	{#if showHistoryOption}
		<label class="check">
			<input type="checkbox" bind:checked={withHistory} />
			<span>{historyLabel}</span>
		</label>
	{/if}
	<form
		class="field"
		onsubmit={(e) => {
			e.preventDefault();
			confirm();
		}}
	>
		<label class="field">
			<span>{tr.del.typeToConfirm}</span>
			<div class="row">
				<input class="input" bind:value={typed} autocomplete="off" spellcheck="false" data-autofocus />
				<button type="button" class="btn btn-sm btn-ghost" onclick={copyWord}>
					{copied ? tr.del.copied : tr.del.copyWord}
				</button>
			</div>
		</label>
	</form>
	{#snippet footer()}
		<button type="button" class="btn" onclick={onCancel}>{tr.common.cancel}</button>
		<button type="button" class="btn btn-danger" disabled={!ready || busy} onclick={confirm}>{confirmLabel}</button>
	{/snippet}
</Modal>

<style>
	.desc {
		color: var(--muted);
	}
	.row {
		display: flex;
		gap: 8px;
		align-items: center;
	}
	.check {
		display: flex;
		gap: 8px;
		align-items: flex-start;
		padding: 10px 12px;
		border: 1px solid var(--danger);
		background: var(--danger-soft);
		border-radius: 6px;
	}
	.check input {
		margin-top: 3px;
	}
</style>
