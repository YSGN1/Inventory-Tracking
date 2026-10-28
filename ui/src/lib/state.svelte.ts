import type { AppStatus } from './bindings/AppStatus';
import { ApiError, toApiError } from './api';

/** Oturum durumu (app_status). */
export const session = $state<{ status: AppStatus | null }>({ status: null });

export const isAdmin = () => session.status?.user?.role === 'admin';

export type Toast = {
	id: number;
	message: string;
	kind: 'success' | 'error';
	action?: { label: string; run: () => void | Promise<void> };
};

export const toasts = $state<{ items: Toast[] }>({ items: [] });
let nextId = 1;

export function toast(message: string, kind: Toast['kind'] = 'success', action?: Toast['action']) {
	const id = nextId++;
	toasts.items.push({ id, message, kind, action });
	setTimeout(() => dismiss(id), action ? 8000 : kind === 'error' ? 6000 : 3000);
}

export function dismiss(id: number) {
	const i = toasts.items.findIndex((t) => t.id === id);
	if (i >= 0) toasts.items.splice(i, 1);
}

/** Oturum düştüyse giriş ekranına dönmek için layout bu kancayı kurar. */
export const hooks: { onUnauthenticated: () => void } = { onUnauthenticated: () => {} };

/** Hata olursa mesajı gösterir ve `undefined` döner. */
export async function attempt<T>(p: Promise<T>): Promise<T | undefined> {
	try {
		return await p;
	} catch (e) {
		const err = toApiError(e);
		// Oturum düştüyse giriş ekranı kendini anlatır; her istek için uyarı basılmaz.
		if (err.code === 'unauthenticated') hooks.onUnauthenticated();
		else toast(err.message, 'error');
		return undefined;
	}
}

export function isConflict(e: unknown): boolean {
	return e instanceof ApiError && e.code === 'conflict';
}

/** Ctrl/Cmd+N ile açılan hareket penceresi. */
export const ui = $state<{ movementOpen: boolean; movementVariantId: string | null; refreshKey: number }>({
	movementOpen: false,
	movementVariantId: null,
	refreshKey: 0
});

export function openMovement(variantId: string | null = null) {
	ui.movementVariantId = variantId;
	ui.movementOpen = true;
}

/** Bir veri değiştiğinde açık sayfalar yeniden yüklensin. */
export function bumpRefresh() {
	ui.refreshKey++;
}
