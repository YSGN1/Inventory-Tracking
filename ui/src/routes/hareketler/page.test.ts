import { render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({ listMovements: vi.fn(), listFirms: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), api }));

import Page from './+page.svelte';

describe('Hareketler page', () => {
	it('loads the list once, not in a loop', async () => {
		api.listMovements.mockResolvedValue([]);
		api.listFirms.mockResolvedValue([]);
		globalThis.IntersectionObserver ??= class {
			observe() {}
			disconnect() {}
		} as unknown as typeof IntersectionObserver;
		render(Page);
		await new Promise((r) => setTimeout(r, 100));
		expect(api.listMovements).toHaveBeenCalledTimes(1);
	});
});
