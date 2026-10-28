<script lang="ts">
	import { api, toApiError } from '$lib/api';
	import type { Firm } from '$lib/bindings/Firm';
	import type { MovementKind } from '$lib/bindings/MovementKind';
	import type { MovementView } from '$lib/bindings/MovementView';
	import type { NewMovement } from '$lib/bindings/NewMovement';
	import type { StockSummaryRow } from '$lib/bindings/StockSummaryRow';
	import { fmtNum, fmtSigned } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { bumpRefresh, toast } from '$lib/state.svelte';
	import Modal from './Modal.svelte';

	let {
		open,
		variantId = null,
		correcting = null,
		onClose
	}: {
		open: boolean;
		variantId?: string | null;
		/** Düzeltilen hareket: kayıt ters kayıt + yeni kayıt olarak yapılır. */
		correcting?: MovementView | null;
		onClose: () => void;
	} = $props();

	type Tab = Exclude<MovementKind, 'reversal'>;
	const TABS: { kind: Tab; label: string }[] = [
		{ kind: 'in', label: tr.movements.in },
		{ kind: 'out', label: tr.movements.out },
		{ kind: 'adjust', label: tr.movements.adjust }
	];

	let rows = $state<StockSummaryRow[]>([]);
	let firms = $state<Firm[]>([]);
	let kind = $state<Tab>('in');
	let selected = $state('');
	let quantity = $state<number | null>(null);
	let firmId = $state('');
	let printed = $state(false);
	let note = $state('');
	let busy = $state(false);
	// Pencere her açıldığında bir kez üretilir; çift tıklama aynı anahtarı gönderir.
	let key = $state('');

	const newKey = () =>
		globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}-0000-0000-000000000000`;

	$effect(() => {
		if (!open) return;
		key = newKey();
		busy = false;
		quantity = correcting ? correcting.quantity : null;
		kind = correcting && correcting.kind !== 'reversal' ? correcting.kind : 'in';
		firmId = correcting?.firm_id ?? '';
		printed = correcting?.printed ?? false;
		note = '';
		const want = correcting?.variant_id ?? variantId ?? '';
		Promise.all([api.stockSummary(), api.listFirms()])
			.then(([r, f]) => {
				rows = r;
				firms = f;
				selected = want && r.some((x) => x.variant_id === want) ? want : '';
			})
			.catch((e) => toast(toApiError(e).message, 'error'));
	});

	const current = $derived(rows.find((r) => r.variant_id === selected)?.current ?? 0);
	const qty = $derived(quantity === null || Number.isNaN(Number(quantity)) ? null : Number(quantity));
	const overStock = $derived(kind === 'out' && qty !== null && qty > current && !correcting);
	const valid = $derived(
		!!selected && qty !== null && Number.isInteger(qty) && (kind === 'adjust' ? qty >= 0 : qty > 0)
	);

	function label(r: StockSummaryRow) {
		return `${r.product_name} — ${r.variant_name} (${tr.movements.current.toLowerCase()} ${fmtNum(r.current)})`;
	}

	async function submit(e?: Event) {
		e?.preventDefault();
		if (busy || !valid || qty === null) return;
		busy = true;
		const m: NewMovement = {
			variant_id: selected,
			kind,
			quantity: qty,
			firm_id: kind === 'out' && firmId ? firmId : null,
			printed: kind === 'out' ? printed : null,
			note: note.trim() || null,
			idempotency_key: key,
			source: 'local'
		};
		try {
			if (correcting) {
				await api.correctMovement(correcting.id, m);
				toast(tr.movements.corrected);
			} else {
				await api.recordMovement(m);
				toast(tr.movements.recorded);
			}
			bumpRefresh();
			onClose();
		} catch (err) {
			toast(toApiError(err).message, 'error');
			busy = false;
		}
	}
</script>

<Modal {open} title={correcting ? tr.movements.correct : tr.nav.newMovement} width={520} {onClose}>
	<div class="tabs" role="tablist">
		{#each TABS as t (t.kind)}
			<button
				type="button"
				role="tab"
				aria-selected={kind === t.kind}
				class:on={kind === t.kind}
				onclick={() => (kind = t.kind)}>{t.label}</button
			>
		{/each}
	</div>

	<form class="grid gap-3" onsubmit={submit}>
		<label class="field">
			<span>{tr.movements.variant}</span>
			<select class="input" bind:value={selected} aria-label={tr.movements.variant} disabled={!!correcting}>
				<option value="" disabled>{tr.movements.chooseVariant}</option>
				{#each rows as r (r.variant_id)}
					<option value={r.variant_id}>{label(r)}</option>
				{/each}
			</select>
		</label>

		<div class="grid grid-cols-2 gap-3 items-end">
			<label class="field">
				<span>{kind === 'adjust' ? tr.movements.counted : tr.movements.quantity}</span>
				<input
					class="input num qty"
					type="number"
					min="0"
					step="1"
					inputmode="numeric"
					bind:value={quantity}
					aria-invalid={overStock}
					data-autofocus
				/>
			</label>
			{#if kind === 'adjust' && selected}
				<p class="diff">
					{tr.movements.diff}
					<strong data-testid="diff" class="num">{qty === null ? '—' : fmtSigned(qty - current)}</strong>
				</p>
			{/if}
		</div>
		{#if overStock}
			<p class="warn" role="alert">{tr.movements.overStock(current)}</p>
		{/if}

		{#if kind === 'out'}
			<div class="grid grid-cols-2 gap-3 items-end">
				<label class="field">
					<span>{tr.movements.firm}</span>
					<select class="input" bind:value={firmId}>
						<option value="">{tr.movements.noFirm}</option>
						{#each firms as f (f.id)}
							<option value={f.id}>{f.name}</option>
						{/each}
					</select>
				</label>
				<label class="switch">
					<input type="checkbox" bind:checked={printed} />
					<span>{tr.movements.printed}</span>
				</label>
			</div>
		{/if}

		<label class="field">
			<span>{tr.common.note}</span>
			<input class="input" bind:value={note} maxlength="500" />
		</label>
	</form>

	{#snippet footer()}
		<button type="button" class="btn" onclick={onClose}>{tr.common.cancel}</button>
		<button type="button" class="btn btn-primary" disabled={busy || !valid} onclick={submit}>{tr.movements.record}</button>
	{/snippet}
</Modal>

<style>
	.tabs {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		overflow: hidden;
	}
	.tabs button {
		padding: 7px 8px;
		background: var(--surface);
		font-weight: 500;
	}
	.tabs button + button {
		border-left: 1px solid var(--line-strong);
	}
	.tabs button.on {
		background: var(--ink);
		color: var(--paper);
	}
	.qty {
		height: 44px;
		font-size: 22px;
		font-weight: 600;
	}
	.diff {
		display: grid;
		color: var(--muted);
		font-size: 12.5px;
	}
	.diff strong {
		font-size: 22px;
		color: var(--ink);
		text-align: left;
	}
	.warn {
		color: var(--danger);
		font-size: 13px;
	}
	.switch {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 34px;
	}
</style>
