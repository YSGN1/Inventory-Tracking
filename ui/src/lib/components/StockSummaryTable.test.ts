import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import StockSummaryTable from './StockSummaryTable.svelte';

const rows = [
	{ variant_id: 'v1', product_id: 'p1', product_name: 'Kupa', variant_name: 'Standart', total_in: 10, total_out: 6, total_adjust: -1, current: 3, min_stock: 3 },
	{ variant_id: 'v2', product_id: 'p2', product_name: 'Bardak', variant_name: 'Büyük', total_in: 5, total_out: 0, total_adjust: 0, current: 5, min_stock: 0 }
];

describe('StockSummaryTable', () => {
	it('has exactly the five summary columns', () => {
		render(StockSummaryTable, { props: { rows } });
		const headers = screen.getAllByRole('columnheader').map((h) => h.textContent?.trim());
		expect(headers).toEqual(['Ürün / Varyant', 'Giren', 'Çıkan', 'Düzeltme', 'Mevcut']);
	});

	it('shows values and highlights rows at or below threshold', () => {
		render(StockSummaryTable, { props: { rows } });
		const [, kupa, bardak] = screen.getAllByRole('row');
		expect(within(kupa).getByText('−1')).toBeInTheDocument();
		expect(kupa).toHaveClass('critical');
		expect(bardak).not.toHaveClass('critical');
	});
});
