import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
	preprocess: vitePreprocess(),
	kit: {
		// Tauri içinde tek sayfa uygulaması: her yol index.html'e düşer.
		adapter: adapter({ fallback: 'index.html' })
	}
};
