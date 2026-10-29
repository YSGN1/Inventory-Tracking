import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({ installUpdate: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import UpdateBanner from './UpdateBanner.svelte';

describe('UpdateBanner', () => {
	it('shows nothing when up to date', () => {
		render(UpdateBanner, { props: { info: { current: '0.1.0', available: null, notes: null } } });
		expect(screen.queryByRole('button')).toBeNull();
	});

	it('installs after confirmation', async () => {
		const user = userEvent.setup();
		api.installUpdate.mockResolvedValue(undefined);
		render(UpdateBanner, { props: { info: { current: '0.1.0', available: '0.2.0', notes: 'Hata düzeltmeleri' } } });
		expect(screen.getByText('Yeni sürüm 0.2.0 hazır')).toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Güncelle' }));
		expect(screen.getByText('Hata düzeltmeleri')).toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Güncelle ve yeniden başlat' }));
		expect(api.installUpdate).toHaveBeenCalledOnce();
	});
});
