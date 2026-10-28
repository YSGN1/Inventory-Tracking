import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';

function setup(showHistoryOption = false) {
	const onConfirm = vi.fn();
	const onCancel = vi.fn();
	render(DeleteConfirmDialog, {
		props: { open: true, title: 'Kupa silinsin mi?', description: 'Çöp kutusuna taşınır.', showHistoryOption, onConfirm, onCancel }
	});
	return { onConfirm, onCancel, user: userEvent.setup() };
}

describe('DeleteConfirmDialog', () => {
	it('enables the delete button only for exact SİL', async () => {
		const { user } = setup();
		const input = screen.getByLabelText(/SİL yazın/);
		const button = screen.getByRole('button', { name: 'Sil' });
		expect(button).toBeDisabled();
		for (const bad of ['sil', 'SIL', 'Sil']) {
			await user.clear(input);
			await user.type(input, bad);
			expect(button).toBeDisabled();
		}
		await user.clear(input);
		await user.type(input, ' SİL ');
		expect(button).toBeEnabled();
	});

	it('has no history checkbox unless asked', () => {
		setup(false);
		expect(screen.queryByRole('checkbox')).toBeNull();
	});

	it('calls onConfirm with the word and history flag', async () => {
		const { user, onConfirm } = setup(true);
		await user.type(screen.getByLabelText(/SİL yazın/), 'SİL');
		await user.click(screen.getByRole('button', { name: 'Sil' }));
		expect(onConfirm).toHaveBeenCalledWith('SİL', false);
	});

	it('passes withHistory=true when the box is checked', async () => {
		const { user, onConfirm } = setup(true);
		await user.click(screen.getByRole('checkbox'));
		await user.type(screen.getByLabelText(/SİL yazın/), 'SİL');
		await user.click(screen.getByRole('button', { name: 'Sil' }));
		expect(onConfirm).toHaveBeenCalledWith('SİL', true);
	});
});
