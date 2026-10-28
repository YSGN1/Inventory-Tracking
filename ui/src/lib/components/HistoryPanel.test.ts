import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({ auditHistory: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import HistoryPanel from './HistoryPanel.svelte';

describe('HistoryPanel', () => {
	it('lists changes newest first with field diffs and user names', async () => {
		api.auditHistory.mockResolvedValue([
			{
				id: '2', entity: 'product', entity_id: 'p', action: 'update',
				before: { name: 'Kupa', sku: null }, after: { name: 'Büyük Kupa', sku: null },
				user_id: 'u', user_name: 'Ayşe', created_at: 1_767_225_600_000
			},
			{
				id: '1', entity: 'product', entity_id: 'p', action: 'create', before: null, after: { name: 'Kupa' },
				user_id: '00000000-0000-0000-0000-000000000000', user_name: null, created_at: 1_767_225_000_000
			}
		]);
		render(HistoryPanel, { props: { entity: 'product', id: 'p' } });
		expect(await screen.findByText('Ad: Kupa → Büyük Kupa')).toBeInTheDocument();
		expect(screen.getByText('Ayşe')).toBeInTheDocument();
		expect(screen.getByText('Sistem')).toBeInTheDocument();
		expect(screen.getByText('Oluşturuldu')).toBeInTheDocument();
		expect(api.auditHistory).toHaveBeenCalledWith('product', 'p');
	});
});
