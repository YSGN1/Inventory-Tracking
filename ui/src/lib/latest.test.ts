import { describe, expect, it } from 'vitest';
import { latest } from './latest';

describe('latest', () => {
	it('drops responses from superseded requests', async () => {
		const next = latest();
		let resolveOld!: (v: string) => void;
		const old = next(new Promise<string>((r) => (resolveOld = r)));
		const fresh = next(Promise.resolve('yeni'));
		resolveOld('eski');
		expect(await fresh).toBe('yeni');
		expect(await old).toBeUndefined();
	});
});
