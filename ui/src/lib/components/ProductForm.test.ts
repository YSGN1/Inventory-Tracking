import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import ProductForm from './ProductForm.svelte';

describe('ProductForm', () => {
	it('disables save while the name is empty', async () => {
		const user = userEvent.setup();
		render(ProductForm, { props: { onSave: vi.fn(), onCancel: vi.fn() } });
		const save = screen.getByRole('button', { name: 'Kaydet' });
		expect(save).toBeDisabled();
		await user.type(screen.getByLabelText('Ad'), '   ');
		expect(save).toBeDisabled();
		await user.type(screen.getByLabelText('Ad'), 'Kupa');
		expect(save).toBeEnabled();
	});

	it('rejects a negative critical threshold', async () => {
		const user = userEvent.setup();
		render(ProductForm, { props: { onSave: vi.fn(), onCancel: vi.fn() } });
		await user.type(screen.getByLabelText('Ad'), 'Kupa');
		await user.click(screen.getByRole('button', { name: 'Varyant ekle' }));
		await user.type(screen.getByLabelText('Varyant adı'), 'Büyük');
		const min = screen.getByLabelText('Kritik eşik');
		await user.clear(min);
		await user.type(min, '-2');
		expect(screen.getByText('Kritik eşik negatif olamaz')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Kaydet' })).toBeDisabled();
	});

	it('sends drafts on save', async () => {
		const user = userEvent.setup();
		const onSave = vi.fn().mockResolvedValue(undefined);
		render(ProductForm, { props: { onSave, onCancel: vi.fn() } });
		await user.type(screen.getByLabelText('Ad'), 'Tişört');
		await user.click(screen.getByRole('button', { name: 'Varyant ekle' }));
		await user.type(screen.getByLabelText('Varyant adı'), 'M');
		await user.click(screen.getByRole('button', { name: 'Kaydet' }));
		expect(onSave).toHaveBeenCalledWith({
			name: 'Tişört',
			sku: '',
			description: '',
			variants: [{ name: 'M', sku: null, min_stock: 0 }]
		});
	});

	it('shows a refresh button after a conflict', async () => {
		const user = userEvent.setup();
		const onRefresh = vi.fn();
		const onSave = vi.fn().mockRejectedValue(new ApiError('conflict', 'Kayıt başka biri tarafından değiştirildi, lütfen yenileyin'));
		render(ProductForm, {
			props: {
				initial: { name: 'Kupa', sku: null, description: null },
				onSave,
				onCancel: vi.fn(),
				onRefresh
			}
		});
		await user.click(screen.getByRole('button', { name: 'Kaydet' }));
		const refresh = await screen.findByRole('button', { name: 'Yenile' });
		await user.click(refresh);
		expect(onRefresh).toHaveBeenCalled();
	});
});
