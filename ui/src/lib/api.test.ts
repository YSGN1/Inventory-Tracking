import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { ApiError, api, toApiError } from './api';

describe('api', () => {
	beforeEach(() => invoke.mockReset());

	it('converts backend error objects to ApiError', async () => {
		invoke.mockImplementationOnce(async () => { throw { code: 'insufficient_stock', message: 'Yetersiz stok: mevcut 3, istenen 5' }; });
		let err: unknown;
		try {
			await api.recordMovement({} as never);
		} catch (e) {
			err = e;
		}
		expect(err instanceof ApiError).toBe(true);
		expect((err as ApiError).code).toBe('insufficient_stock');
		expect((err as ApiError).message).toBe('Yetersiz stok: mevcut 3, istenen 5');
	});

	it('wraps plain strings and unknown values', () => {
		expect(toApiError('patladı')).toMatchObject({ code: 'unknown', message: 'patladı' });
		expect(toApiError(42).code).toBe('unknown');
	});

	it('passes camelCase arguments to the command', async () => {
		invoke.mockResolvedValue(undefined);
		await api.trashProduct('p1', 3, 'SİL');
		expect(invoke).toHaveBeenCalledWith('trash_product', { id: 'p1', expectedVersion: 3, confirm: 'SİL' });
	});

	it('fills query defaults', async () => {
		invoke.mockResolvedValue([]);
		await api.listMovements({ variant_id: 'v1' });
		expect(invoke).toHaveBeenCalledWith('list_movements', {
			query: expect.objectContaining({ variant_id: 'v1', limit: 200, before: null, kind: null })
		});
	});
});
