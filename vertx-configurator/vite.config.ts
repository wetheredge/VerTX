import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import autoprefixer from 'autoprefixer';
import type { UserConfig } from 'vite';
import solid from 'vite-plugin-solid';

const ports = {
	port: 8001,
	strictPort: true,
};

const config: UserConfig = {
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
	server: {
		...ports,
		hmr: {
			clientPort: ports.port,
		},
	},
	preview: ports,
	cacheDir: '.vite',
};

if (process.env.VITE_TARGET === 'simulator') {
	config.base = '/configurator';

	// Use default <base>/assets/* path for assets
	// biome-ignore lint/performance/noDelete:
	delete config.build?.rollupOptions?.output;
}

// biome-ignore lint/style/noDefaultExport: Required by Vite
export default config;
