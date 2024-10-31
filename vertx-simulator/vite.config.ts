import { defineConfig } from 'vite';

const ports = {
	port: 8000,
	strictPort: true,
};

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default defineConfig({
	build: {
		target: 'esnext',
	},
	server: {
		...ports,
		proxy: {
			'/configurator': 'http://localhost:8001',
		},
	},
	preview: ports,
	cacheDir: '.vite',
});
