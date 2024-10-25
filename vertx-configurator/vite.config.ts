import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import autoprefixer from 'autoprefixer';
import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

const ports = {
	port: 8001,
	strictPort: true,
};

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default defineConfig({
	build: {
		rollupOptions: {
			output: {
				assetFileNames: '[hash].[ext]',
				chunkFileNames: '[hash].js',
				entryFileNames: '[hash].js',
			},
		},
	},
	plugins: [solid(), vanillaExtractPlugin()],
	define: {
		'import.meta.env.CODESPACE_NAME': JSON.stringify(
			process.env.CODESPACE_NAME,
		),
	},
	css: {
		postcss: {
			plugins: [autoprefixer({})],
		},
	},
	server: ports,
	preview: ports,
	cacheDir: '.vite',
});
