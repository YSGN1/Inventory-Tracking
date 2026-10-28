/** Yalnız en son başlatılan isteğin sonucunu döndürür; eskiler `undefined` olur. */
export function latest() {
	let seq = 0;
	return async <T>(p: Promise<T>): Promise<T | undefined> => {
		const mine = ++seq;
		const v = await p;
		return mine === seq ? v : undefined;
	};
}
