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
	server: ports,
	preview: ports,
	cacheDir: '.vite',
});
