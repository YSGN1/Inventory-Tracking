<script lang="ts">
	import { api } from '$lib/api';
	import type { AuditEntry } from '$lib/bindings/AuditEntry';
	import { fmtDate } from '$lib/format';
	import { tr } from '$lib/i18n/tr';
	import { attempt, ui } from '$lib/state.svelte';

	let { entity, id }: { entity: 'product' | 'variant' | 'firm'; id: string } = $props();

	let entries = $state<AuditEntry[] | null>(null);

	$effect(() => {
		void ui.refreshKey;
		attempt(api.auditHistory(entity, id)).then((r) => (entries = r ?? []));
	});

	const ACTIONS: Record<string, string> = {
		create: 'Oluşturuldu',
		update: 'Güncellendi',
		delete: 'Çöp kutusuna taşındı',
		restore: 'Geri getirildi',
		purge: 'Kalıcı silindi'
	};

	const FIELDS: Record<string, string> = {
		name: tr.common.name,
		sku: tr.common.sku,
		description: tr.products.description,
		image_path: tr.products.image,
		min_stock: tr.products.minStock,
		contact: tr.firms.contact,
		phone: tr.firms.phone,
		note: tr.common.note
	};

	const show = (v: unknown) => (v === null || v === undefined || v === '' ? '—' : String(v));

	function changes(e: AuditEntry): string[] {
		const b = (e.before ?? {}) as Record<string, unknown>;
		const a = (e.after ?? {}) as Record<string, unknown>;
		if (e.action !== 'update') {
			if (e.action === 'purge' && typeof a.message === 'string') return [a.message];
			return [];
		}
		return Object.keys(FIELDS)
			.filter((k) => k in a && JSON.stringify(a[k]) !== JSON.stringify(b[k]))
			.map((k) => (k === 'image_path' ? `${FIELDS[k]} değişti` : `${FIELDS[k]}: ${show(b[k])} → ${show(a[k])}`));
	}
</script>

{#if entries === null}
	<p class="text-muted">{tr.common.loading}</p>
{:else}
	<ol class="log">
		{#each entries as e (e.id)}
			<li>
				<time class="text-muted">{fmtDate(e.created_at)}</time>
				<div>
					<strong>{ACTIONS[e.action] ?? e.action}</strong>
					{#each changes(e) as c, i (i)}<p>{c}</p>{/each}
				</div>
				<span class="who">{e.user_name ?? tr.common.system}</span>
			</li>
		{/each}
	</ol>
{/if}

<style>
	.log {
		display: grid;
		border: 1px solid var(--line);
		border-radius: 8px;
		background: var(--surface);
	}
	li {
		display: grid;
		grid-template-columns: 140px 1fr auto;
		gap: 16px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--line);
	}
	li:last-child {
		border-bottom: 0;
	}
	time {
		font-variant-numeric: tabular-nums;
	}
	strong {
		font-weight: 500;
	}
	p {
		color: var(--muted);
	}
	.who {
		color: var(--muted);
	}
</style>
