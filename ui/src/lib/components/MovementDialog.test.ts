import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
	stockSummary: vi.fn(),
	listFirms: vi.fn(),
	recordMovement: vi.fn(),
	correctMovement: vi.fn()
}));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import { ApiError } from '$lib/api';
import { toasts } from '$lib/state.svelte';
import MovementDialog from './MovementDialog.svelte';

const row = { variant_id: 'v1', product_id: 'p1', product_name: 'Kupa', variant_name: 'Standart', total_in: 10, total_out: 0, total_adjust: 0, current: 10, min_stock: 0 };

function setup() {
	const onClose = vi.fn();
	render(MovementDialog, { props: { open: true, variantId: 'v1', onClose } });
	return { onClose, user: userEvent.setup() };
}

describe('MovementDialog', () => {
	beforeEach(() => {
		toasts.items.splice(0);
		Object.values(api).forEach((f) => f.mockReset());
		api.stockSummary.mockResolvedValue([row]);
		api.listFirms.mockResolvedValue([{ id: 'f1', name: 'Öz Matbaa', contact: null, phone: null, note: null, row_version: 1, deleted_at: null }]);
	});

	it('warns when out exceeds stock and shows the backend error', async () => {
		const { user } = setup();
		await user.click(await screen.findByRole('tab', { name: 'Çıkış' }));
		await waitFor(() => expect(screen.getByRole('combobox', { name: 'Ürün / Varyant' })).toHaveValue('v1'));
		await user.type(screen.getByLabelText('Adet'), '12');
		expect(screen.getByText('Mevcut stok 10; daha fazla çıkış yapılamaz')).toBeInTheDocument();
		api.recordMovement.mockImplementationOnce(async () => {
			throw new ApiError('insufficient_stock', 'Yetersiz stok: mevcut 10, istenen 12');
		});
		await user.click(screen.getByRole('button', { name: 'Kaydet' }));
		await waitFor(() => expect(toasts.items.at(-1)?.message).toBe('Yetersiz stok: mevcut 10, istenen 12'));
		expect(toasts.items.at(-1)?.kind).toBe('error');
	});

	it('previews the adjustment difference as counted minus current', async () => {
		const { user } = setup();
		await user.click(await screen.findByRole('tab', { name: 'Sayım / Düzeltme' }));
		await waitFor(() => expect(screen.getByRole('combobox', { name: 'Ürün / Varyant' })).toHaveValue('v1'));
		await user.type(screen.getByLabelText('Sayılan adet'), '7');
		expect(screen.getByTestId('diff')).toHaveTextContent('−3');
	});

	it('sends one request with a stable idempotency key on double click', async () => {
		const { user, onClose } = setup();
		await waitFor(() => expect(screen.getByRole('combobox', { name: 'Ürün / Varyant' })).toHaveValue('v1'));
		await user.type(screen.getByLabelText('Adet'), '3');
		let resolve!: (v: unknown) => void;
		api.recordMovement.mockImplementation(() => new Promise((r) => (resolve = r)));
		const save = screen.getByRole('button', { name: 'Kaydet' });
		await user.dblClick(save);
		expect(save).toBeDisabled();
		resolve({});
		await waitFor(() => expect(onClose).toHaveBeenCalled());
		const keys = api.recordMovement.mock.calls.map((c) => c[0].idempotency_key);
		expect(keys.length).toBe(1);
		expect(keys[0]).toMatch(/[0-9a-f-]{36}/);
		expect(api.recordMovement.mock.calls[0][0]).toMatchObject({ variant_id: 'v1', kind: 'in', quantity: 3, firm_id: null, printed: null });
	});

	it('out with firm sends printed flag', async () => {
		const { user } = setup();
		await user.click(await screen.findByRole('tab', { name: 'Çıkış' }));
		await waitFor(() => expect(screen.getByRole('combobox', { name: 'Ürün / Varyant' })).toHaveValue('v1'));
		await user.type(screen.getByLabelText('Adet'), '2');
		await user.selectOptions(screen.getByLabelText('Firma'), 'f1');
		await user.click(screen.getByLabelText('Baskı yapıldı'));
		api.recordMovement.mockResolvedValue({});
		await user.click(screen.getByRole('button', { name: 'Kaydet' }));
		await waitFor(() =>
			expect(api.recordMovement).toHaveBeenCalledWith(expect.objectContaining({ kind: 'out', quantity: 2, firm_id: 'f1', printed: true }))
		);
	});
});
