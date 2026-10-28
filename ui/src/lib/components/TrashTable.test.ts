import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({ purgeItem: vi.fn(), restoreItem: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import TrashTable, { daysLeft } from './TrashTable.svelte';

const DAY = 86_400_000;
const now = 1_800_000_000_000;
const items = [
	{ kind: 'product' as const, id: 'p1', name: 'Kupa', parent_name: null, deleted_at: now - DAY, purge_at: now + 29 * DAY - 1, stock_qty: 7, row_version: 2 },
	{ kind: 'variant' as const, id: 'v1', name: 'Büyük', parent_name: 'Bardak', deleted_at: now, purge_at: now + 2.2 * DAY, stock_qty: 0, row_version: 3 }
];

describe('TrashTable', () => {
	beforeEach(() => {
		api.purgeItem.mockReset().mockResolvedValue({ removed_entities: 2, removed_movements: 0, note: null });
		api.restoreItem.mockReset().mockResolvedValue(undefined);
	});

	it('computes remaining days with ceil', () => {
		expect(daysLeft(now + 29 * DAY - 1, now)).toBe(29);
		expect(daysLeft(now + 2.2 * DAY, now)).toBe(3);
		expect(daysLeft(now - 5, now)).toBe(0);
		render(TrashTable, { props: { items, now } });
		expect(screen.getByText('29 gün')).toBeInTheDocument();
		expect(screen.getByText('3 gün')).toBeInTheDocument();
		expect(screen.getByText('Bardak / Büyük')).toBeInTheDocument();
	});

	it('purges with history when the box is checked', async () => {
		const user = userEvent.setup();
		render(TrashTable, { props: { items, now } });
		const row = screen.getAllByRole('row')[1];
		await user.click(within(row).getByRole('button', { name: 'Kalıcı sil' }));
		await user.click(screen.getByRole('checkbox'));
		await user.type(screen.getByLabelText(/SİL yazın/), 'SİL');
		await user.click(screen.getByRole('button', { name: 'Sil' }));
		expect(api.purgeItem).toHaveBeenCalledWith('product', 'p1', 'SİL', true);
	});

	it('restore uses the row version', async () => {
		const user = userEvent.setup();
		render(TrashTable, { props: { items, now } });
		await user.click(within(screen.getAllByRole('row')[2]).getByRole('button', { name: 'Geri getir' }));
		expect(api.restoreItem).toHaveBeenCalledWith('variant', 'v1', 3);
	});
});
