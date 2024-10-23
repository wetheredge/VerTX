import { defineConfig } from 'vite';

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default defineConfig({
	build: {
		target: 'esnext',
	},
	cacheDir: '.vite',
});
