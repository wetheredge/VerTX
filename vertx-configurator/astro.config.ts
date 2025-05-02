import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import { minify } from '@zokki/astro-minify';
import type { AstroUserConfig } from 'astro';
import { envField } from 'astro/config';

const port = 8001;
const isSimulator = process.env.VERTX_SIMULATOR === 'true';

const assetsPrefix = isSimulator ? 'assets/' : '_';

const config = {
	integrations: [minify({ minifyCid: false })],

	base: isSimulator && import.meta.env.PROD ? '/configurator' : '',
	env: {
		schema: {
			// biome-ignore lint/style/useNamingConvention:
			VERTX_SIMULATOR: envField.boolean({
				context: 'client',
				access: 'public',
				default: false,
			}),
		},
	},
	devToolbar: {
		enabled: !isSimulator,
	},

	build: {
		format: 'file',
	},
	scopedStyleStrategy: 'class',
	server: { port },
	cacheDir: '.astro',

	vite: {
		plugins: [vanillaExtractPlugin()],
		build: {
			rollupOptions: {
				output: {
					assetFileNames: `${assetsPrefix}[hash].[ext]`,
					chunkFileNames: `${assetsPrefix}[hash].js`,
					entryFileNames: `${assetsPrefix}[hash].js`,
				},
			},
		},
		server: { strictPort: true },
		cacheDir: '.vite',
	},
} satisfies AstroUserConfig;

// biome-ignore lint/style/noDefaultExport:
export default config;
