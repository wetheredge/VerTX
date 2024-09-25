import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default defineConfig({
	build: {
		target: 'esnext',
	},
	worker: {
		format: 'es',
		plugins: () => [wasm()],
	},
	cacheDir: '.vite',
});
