import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	compilerOptions: {
		runes: true
	},
	kit: {
		// SPA: todas as rotas caem em index.html (ssr = false no layout raiz)
		adapter: adapter({ fallback: 'index.html' })
	}
};

export default config;
