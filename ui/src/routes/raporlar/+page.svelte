<script lang="ts">
	import { save } from '@tauri-apps/plugin-dialog';
	import { api } from '$lib/api';
	import type { DeliveryReport } from '$lib/bindings/DeliveryReport';
	import type { Firm } from '$lib/bindings/Firm';
	import DataTable from '$lib/components/DataTable.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import { dayEnd, dayStart, fmtDate, fmtNum } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, toast, ui } from '$lib/state.svelte';

	let report = $state<DeliveryReport | null>(null);
	let firms = $state<Firm[]>([]);
	let firmId = $state('');
	let from = $state('');
	let to = $state('');
	let printed = $state<'' | 'yes' | 'no'>('');

	const query = () => ({
		firm_id: firmId || null,
		from_ms: dayStart(from),
		to_ms: dayEnd(to),
		printed: printed === '' ? null : printed === 'yes'
	});

	$effect(() => {
		void ui.refreshKey;
		void [firmId, from, to, printed];
		attempt(api.firmDeliveryReport(query())).then((r) => r && (report = r));
	});

	$effect(() => {
		attempt(api.listFirms()).then((f) => f && (firms = f));
	});

	const grand = $derived(
		(report?.totals ?? []).reduce(
			(a, t) => ({ printed: a.printed + t.printed_qty, unprinted: a.unprinted + t.unprinted_qty, total: a.total + t.total_qty }),
			{ printed: 0, unprinted: 0, total: 0 }
		)
	);

	async function exportCsv() {
		const stamp = new Date().toISOString().slice(0, 10);
		const dest = await save({ defaultPath: `firma-teslim-${stamp}.csv`, filters: [{ name: 'CSV', extensions: ['csv'] }] });
		if (!dest) return;
		const n = await attempt(api.exportDeliveryCsv(query(), dest));
		if (n !== undefined) toast(tr.reports.exported(n));
	}
</script>

<header class="head">
	<h1>{tr.reports.delivery}</h1>
	<label class="f">
		<span>{tr.movements.firm}</span>
		<select class="input" bind:value={firmId}>
			<option value="">{tr.common.all}</option>
			{#each firms as f (f.id)}<option value={f.id}>{f.name}</option>{/each}
		</select>
	</label>
	<label class="f"><span>{tr.common.from}</span><input class="input" type="date" bind:value={from} /></label>
	<label class="f"><span>{tr.common.to}</span><input class="input" type="date" bind:value={to} /></label>
	<label class="f">
		<span>{tr.reports.printFilter}</span>
		<select class="input" bind:value={printed}>
			<option value="">{tr.common.all}</option>
			<option value="yes">{tr.reports.printed}</option>
			<option value="no">{tr.reports.unprinted}</option>
		</select>
	</label>
	<button class="btn" onclick={exportCsv} disabled={!report?.rows.length}>{tr.reports.exportCsv}</button>
</header>

{#if report === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else if report.rows.length === 0}
	<EmptyState message={tr.reports.empty} />
{:else}
	<section class="totals">
		<DataTable headers={[tr.movements.firm, tr.reports.printed, tr.reports.unprinted, tr.reports.total]} numeric={[1, 2, 3]}>
			{#each report.totals as t (t.firm_name)}
				<tr>
					<td class="font-medium">{t.firm_name}</td>
					<td class="num">{fmtNum(t.printed_qty)}</td>
					<td class="num">{fmtNum(t.unprinted_qty)}</td>
					<td class="num font-semibold">{fmtNum(t.total_qty)}</td>
				</tr>
			{/each}
			{#if report.totals.length > 1}
				<tr class="grand">
					<td>{tr.reports.total}</td>
					<td class="num">{fmtNum(grand.printed)}</td>
					<td class="num">{fmtNum(grand.unprinted)}</td>
					<td class="num">{fmtNum(grand.total)}</td>
				</tr>
			{/if}
		</DataTable>
	</section>

	<DataTable
		headers={[tr.movements.date, tr.movements.firm, tr.movements.variant, tr.movements.quantity, tr.reports.printFilter]}
		numeric={[3]}
	>
		{#each report.rows as r (r.movement_id)}
			<tr class:deleted={r.entity_deleted || r.firm_deleted}>
				<td class="text-muted whitespace-nowrap">{fmtDate(r.date_ms)}</td>
				<td>{r.firm_name}</td>
				<td>{r.product_name} <span class="text-muted">— {r.variant_name}</span></td>
				<td class="num">{fmtNum(r.quantity)}</td>
				<td>{r.printed ? tr.common.yes : tr.common.no}</td>
			</tr>
		{/each}
	</DataTable>
{/if}

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
	.totals {
		max-width: 640px;
		margin-bottom: 20px;
	}
	.grand td {
		font-weight: 600;
		border-top: 1px solid var(--line-strong);
	}
	tr.deleted td {
		opacity: 0.55;
	}
</style>
