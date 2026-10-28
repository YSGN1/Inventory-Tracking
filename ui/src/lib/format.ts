const dateFmt = new Intl.DateTimeFormat('tr-TR', {
	day: '2-digit',
	month: '2-digit',
	year: 'numeric',
	hour: '2-digit',
	minute: '2-digit'
});
const dayFmt = new Intl.DateTimeFormat('tr-TR', { day: '2-digit', month: '2-digit', year: 'numeric' });
const numFmt = new Intl.NumberFormat('tr-TR');

export const fmtDate = (ms: number) => dateFmt.format(new Date(ms));
export const fmtDay = (ms: number) => dayFmt.format(new Date(ms));
export const fmtNum = (n: number) => numFmt.format(n);
// Eksi için tipografik eksi (−) kullanılır; tabloda tire ile karışmasın.
export const fmtSigned = (n: number) => (n > 0 ? `+${numFmt.format(n)}` : n < 0 ? `−${numFmt.format(-n)}` : '0');

/** Türkçe kurala göre baş harf (i → İ). */
export const initial = (s: string) => (s.trim()[0] ?? '?').toLocaleUpperCase('tr-TR');

/** `<input type="date">` değerini gün başı/sonu epoch ms'ye çevirir (yerel saat). */
export function dayStart(v: string): number | null {
	return v ? new Date(`${v}T00:00:00`).getTime() : null;
}
export function dayEnd(v: string): number | null {
	return v ? new Date(`${v}T23:59:59.999`).getTime() : null;
}

/** Tarayıcı depolaması olmayabilir (gizli pencere vb.); her erişim korunur. */
export function loadPref(key: string, fallback: string): string {
	try {
		return localStorage.getItem(key) ?? fallback;
	} catch {
		return fallback;
	}
}
export function savePref(key: string, value: string) {
	try {
		localStorage.setItem(key, value);
	} catch {
		/* tercih saklanamadı */
	}
}

export function debounce<A extends unknown[]>(fn: (...a: A) => void, ms: number) {
	let t: ReturnType<typeof setTimeout> | undefined;
	return (...a: A) => {
		clearTimeout(t);
		t = setTimeout(() => fn(...a), ms);
	};
}
