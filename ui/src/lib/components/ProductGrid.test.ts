import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProductWithVariants } from '$lib/bindings/ProductWithVariants';

const api = vi.hoisted(() => ({ trashProduct: vi.fn(), restoreItem: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import ProductGrid from './ProductGrid.svelte';
import { trashProductWithUndo } from '$lib/actions';
import { toasts } from '$lib/state.svelte';

const product = (name: string, image_path: string | null = null): ProductWithVariants => ({
	product: { id: name, name, sku: null, description: null, image_path, row_version: 1, created_at: 0, updated_at: 0, deleted_at: null },
	variants: [{ id: `${name}-v`, product_id: name, name: 'Standart', sku: null, min_stock: 0, stock_qty: 4, row_version: 1, deleted_at: null }],
	total_stock: 4
});

describe('ProductGrid', () => {
	beforeEach(() => {
		toasts.items.splice(0);
		api.trashProduct.mockReset();
		api.restoreItem.mockReset();
	});

	it('shows an initial badge when there is no image', () => {
		render(ProductGrid, { props: { products: [product('ipek eşarp')], images: {}, view: 'grid', onOpen: vi.fn() } });
		expect(screen.getByText('İ')).toBeInTheDocument();
	});

	it('shows the image when loaded', () => {
		render(ProductGrid, {
			props: { products: [product('Kupa', 'Kupa.png')], images: { Kupa: 'data:image/png;base64,AA==' }, view: 'grid', onOpen: vi.fn() }
		});
		expect(screen.getByRole('img', { name: 'Kupa' })).toHaveAttribute('src', 'data:image/png;base64,AA==');
	});

	it('list view shows variant count and total stock', () => {
		render(ProductGrid, { props: { products: [product('Kupa')], images: {}, view: 'list', onOpen: vi.fn() } });
		expect(screen.getByRole('table')).toBeInTheDocument();
		expect(screen.getByText('1 varyant')).toBeInTheDocument();
	});

	it('undo toast after trashing calls restoreItem', async () => {
		api.trashProduct.mockResolvedValue(undefined);
		api.restoreItem.mockResolvedValue(undefined);
		const p = product('Kupa').product;
		await trashProductWithUndo(p, 'SİL');
		expect(api.trashProduct).toHaveBeenCalledWith('Kupa', 1, 'SİL');
		const t = toasts.items.at(-1)!;
		expect(t.message).toBe('Çöp kutusuna taşındı');
		expect(t.action?.label).toBe('Geri al');
		await t.action!.run();
		// trash bumps row_version by one
		expect(api.restoreItem).toHaveBeenCalledWith('product', 'Kupa', 2);
	});

	it('clicking a card opens the product', async () => {
		const onOpen = vi.fn();
		render(ProductGrid, { props: { products: [product('Kupa')], images: {}, view: 'grid', onOpen } });
		await userEvent.setup().click(screen.getByRole('button', { name: /Kupa/ }));
		expect(onOpen).toHaveBeenCalledWith('Kupa');
	});
});
