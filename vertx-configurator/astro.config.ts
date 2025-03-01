import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import type { AstroUserConfig } from 'astro';
import min from 'astro-min';

const port = 8001;
const isSimulator = process.env.VERTX_TARGET === 'simulator';

const assetsPrefix = isSimulator ? 'assets/' : '_';

const config = {
	// biome-ignore lint/style/useNamingConvention:
	integrations: [min({ minify_css: true })],

	experimental: {
		svg: true,
	},

	base: isSimulator ? '/configurator' : '',
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
		cacheDir: '.vite',
	},
} satisfies AstroUserConfig;

// biome-ignore lint/style/noDefaultExport:
export default config;
