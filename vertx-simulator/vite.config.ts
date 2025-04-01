import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const ports = {
	port: 8000,
	strictPort: true,
};

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default defineConfig({
	build: {
		target: 'esnext',
		outDir: fileURLToPath(new URL('../out/simulator', import.meta.url)),
		emptyOutDir: false,
	},
	server: {
		...ports,
		proxy: {
			'/configurator': 'http://localhost:8001',
		},
	},
	preview: ports,
	cacheDir: '../.cache/simulator/vite',
});
