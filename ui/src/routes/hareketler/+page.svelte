<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '$lib/api';
	import type { Firm } from '$lib/bindings/Firm';
	import type { MovementKind } from '$lib/bindings/MovementKind';
	import type { MovementView } from '$lib/bindings/MovementView';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import MovementDialog from '$lib/components/MovementDialog.svelte';
	import { dayEnd, dayStart, fmtDate, fmtNum, fmtSigned } from '$lib/format';
	import { latest } from '$lib/latest';
	import { kindLabel, tr } from '$lib/i18n/tr';
	import { attempt, bumpRefresh, isAdmin, toast, ui } from '$lib/state.svelte';

	const PAGE = 100;
	const newest = latest();

	let rows = $state<MovementView[] | null>(null);
	let firms = $state<Firm[]>([]);
	let kind = $state<MovementKind | ''>('');
	let firmId = $state('');
	let from = $state('');
	let to = $state('');
	let more = $state(false);
	let loading = $state(false);
	let reversing = $state<MovementView | null>(null);
	let reason = $state('');
	let correcting = $state<MovementView | null>(null);
	let sentinel: HTMLElement | undefined = $state();

	const filters = () => ({
		kind: kind || null,
		firm_id: firmId || null,
		from_ms: dayStart(from),
		to_ms: dayEnd(to),
		limit: PAGE
	});

	async function load(reset: boolean) {
		// Sayfa eklemeyi çiftlemeyelim; filtre değişince (reset) yükleme her zaman başlar.
		if (loading && !reset) return;
		loading = true;
		const before = reset ? null : (rows?.at(-1)?.id ?? null);
		const r = await newest(attempt(api.listMovements({ ...filters(), before })));
		loading = false;
		if (!r) return;
		rows = reset ? r : [...(rows ?? []), ...r];
		more = r.length === PAGE;
	}

	$effect(() => {
		void ui.refreshKey;
		void [kind, firmId, from, to];
		// load() kendi durumunu okuyup yazar; izlenirse her bitişte yeniden başlar.
		untrack(() => load(true));
	});

	$effect(() => {
		attempt(api.listFirms()).then((f) => f && (firms = f));
	});

	// Sonsuz kaydırma: liste sonu görününce sonraki sayfa.
	$effect(() => {
		if (!sentinel) return;
		const io = new IntersectionObserver((e) => e[0].isIntersecting && more && load(false));
		io.observe(sentinel);
		return () => io.disconnect();
	});

	async function reverse() {
		if (!reversing || !reason.trim()) return;
		if (await attempt(api.reverseMovement(reversing.id, reason.trim()))) {
			toast(tr.movements.reversed);
			reversing = null;
			reason = '';
			bumpRefresh();
		}
	}

	const canFix = (m: MovementView) => isAdmin() && m.kind !== 'reversal' && !m.reversed_by && !m.entity_deleted;
</script>

<header class="head">
	<h1>{tr.movements.title}</h1>
	<label class="f">
		<span>{tr.movements.kind}</span>
		<select class="input" bind:value={kind}>
			<option value="">{tr.common.all}</option>
			<option value="in">{tr.movements.in}</option>
			<option value="out">{tr.movements.out}</option>
			<option value="adjust">{tr.movements.adjust}</option>
			<option value="reversal">{tr.movements.reversal}</option>
		</select>
	</label>
	<label class="f">
		<span>{tr.movements.firm}</span>
		<select class="input" bind:value={firmId}>
			<option value="">{tr.common.all}</option>
			{#each firms as f (f.id)}<option value={f.id}>{f.name}</option>{/each}
		</select>
	</label>
	<label class="f"><span>{tr.common.from}</span><input class="input" type="date" bind:value={from} /></label>
	<label class="f"><span>{tr.common.to}</span><input class="input" type="date" bind:value={to} /></label>
</header>

{#if rows === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if rows.length === 0}
	<EmptyState message={tr.movements.empty} />
{:else}
	<DataTable
		headers={[tr.movements.date, tr.movements.variant, tr.movements.kind, tr.movements.quantity, tr.movements.firm, tr.movements.user, '']}
		numeric={[3]}
	>
		{#each rows as m (m.id)}
			<tr class:reversed={!!m.reversed_by} class:deleted={m.entity_deleted || m.firm_deleted}>
				<td class="text-muted whitespace-nowrap">{fmtDate(m.created_at)}</td>
				<td>
					<span class="strike">{m.product_name}</span>
					<span class="sub strike">{m.variant_name}</span>
					{#if m.note}<span class="sub">{m.note}</span>{/if}
				</td>
				<td><span class="kind {m.kind}">{kindLabel(m.kind)}</span></td>
				<td class="num strike">
					{m.kind === 'adjust' ? `${fmtNum(m.quantity)} (${fmtSigned(m.delta)})` : fmtSigned(m.delta)}
				</td>
				<td>
					{m.firm_name ?? ''}
					{#if m.kind === 'out' && m.printed !== null}
						<span class="sub">{m.printed ? tr.reports.printed : tr.reports.unprinted}</span>
					{/if}
				</td>
				<td class="text-muted">{m.user_name ?? tr.common.system}</td>
				<td class="actions">
					{#if canFix(m)}
						<button class="btn btn-sm btn-ghost" onclick={() => (correcting = m)}>{tr.movements.correct}</button>
						<button class="btn btn-sm btn-ghost" onclick={() => (reversing = m)}>{tr.movements.reverse}</button>
					{/if}
				</td>
			</tr>
		{/each}
	</DataTable>
	<div bind:this={sentinel} class="py-3">
		{#if more}<button class="btn btn-sm" onclick={() => load(false)} disabled={loading}>{tr.movements.loadMore}</button>{/if}
	</div>
{/if}

<ConfirmDialog
	open={!!reversing}
	title={tr.movements.reverse}
	description={reversing ? `${reversing.product_name} — ${reversing.variant_name}: ${kindLabel(reversing.kind)} ${fmtNum(reversing.quantity)}` : ''}
	confirmLabel={tr.movements.reverse}
	danger
	canConfirm={reason.trim().length > 0}
	onConfirm={reverse}
	onCancel={() => {
		reversing = null;
		reason = '';
	}}
>
	<label class="field">
		<span>{tr.movements.reason}</span>
		<input class="input" bind:value={reason} maxlength="500" />
	</label>
</ConfirmDialog>

<MovementDialog open={!!correcting} {correcting} onClose={() => (correcting = null)} />

<style>
	.head {
		display: flex;
		align-items: end;
		gap: 12px;
		margin-bottom: 18px;
		flex-wrap: wrap;
	}
	h1 {
		font-size: 22px;
		font-weight: 600;
		margin-right: auto;
		align-self: center;
	}
	.f {
		display: grid;
		gap: 2px;
		font-size: 12px;
		color: var(--muted);
	}
	.f .input {
		width: 150px;
	}
	.sub {
		display: block;
		font-size: 12.5px;
		color: var(--muted);
	}
	.kind {
		font-size: 12.5px;
		font-weight: 500;
		padding: 2px 8px;
		border-radius: 999px;
		background: var(--surface-2);
		border: 1px solid var(--line);
	}
	.kind.in {
		color: var(--ok);
		border-color: var(--ok);
	}
	.kind.out {
		color: var(--danger);
		border-color: var(--danger);
	}
	tr.reversed .strike {
		text-decoration: line-through;
		color: var(--muted);
	}
	tr.deleted td {
		opacity: 0.55;
	}
	.actions {
		text-align: right;
		white-space: nowrap;
	}
</style>
