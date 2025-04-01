import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import { minify } from '@zokki/astro-minify';
import type { AstroUserConfig } from 'astro';
import { envField } from 'astro/config';
import browserslist from 'browserslist';
import viteTarget from 'browserslist-to-esbuild';
import { browserslistToTargets as lightningTargets } from 'lightningcss';
import { isSimulator, outDir } from './scripts/utils.ts';

const port = 8001;

const assetsPrefix = isSimulator ? 'assets/' : '_';

const config: AstroUserConfig = {
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
	outDir,

	vite: {
		plugins: [vanillaExtractPlugin()],
		build: {
			target: viteTarget(),
			rollupOptions: {
				output: {
					assetFileNames: `${assetsPrefix}[hash].[ext]`,
					chunkFileNames: `${assetsPrefix}[hash].js`,
					entryFileNames: `${assetsPrefix}[hash].js`,
				},
			},
		},
		css: {
			transformer: 'lightningcss',
			lightningcss: {
				targets: lightningTargets(browserslist()),
			},
			devSourcemap: true,
		},
		resolve: {
			alias: {
				'~': new URL('./src', import.meta.url).pathname,
			},
		},
		server: { strictPort: true },
		cacheDir: '../.cache/configurator/vite',
	},
};

// biome-ignore lint/style/noDefaultExport:
export default config;
