import * as path from 'node:path';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import { minify } from '@zokki/astro-minify';
import type { AstroIntegration, AstroUserConfig } from 'astro';
import { envField } from 'astro/config';
import browserslist from 'browserslist';
import viteTarget from 'browserslist-to-esbuild';
import { Glob, fileURLToPath } from 'bun';
import { browserslistToTargets as lightningTargets } from 'lightningcss';

const port = 8001;

const isSimulator = process.env.VERTX_SIMULATOR === 'true';

const assetsPrefix = isSimulator ? 'assets/' : '_';
const outDir = fileURLToPath(
	new URL(
		`../out/${isSimulator ? 'simulator/' : ''}configurator`,
		import.meta.url,
	),
);

const config: AstroUserConfig = {
	integrations: [
		minify({ minifyCid: false }),
		vertx({
			skip: isSimulator,
			assetsPrefix,
		}),
	],

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
					chunkFileNames: `${assetsPrefix}[hash].mjs`,
					entryFileNames: `${assetsPrefix}[hash].mjs`,
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

type VertxOptions = {
	assetsPrefix: string;
	skip?: boolean;
};
function vertx(options: VertxOptions): undefined | AstroIntegration {
	if (options.skip) {
		return;
	}

	const assetPaths = (outDir: string) =>
		new Glob(`${options.assetsPrefix}**`).scan({ cwd: outDir });

	type AssetBase = {
		route: string;
		path: URL;
	};
	type Asset = {
		route: string;
		file: string;
		mime: string;
		gzip: boolean;
	};

	const baseAssets = new Array<AssetBase>();

	return {
		name: 'vertx',
		hooks: {
			'astro:build:ssr': ({ manifest, logger }) => {
				for (const routeMeta of manifest.routes) {
					const route = routeMeta.routeData.pathname;
					if (route == null) {
						logger.error(
							`Skipping dynamic route: ${routeMeta.routeData.pattern}`,
						);
						continue;
					}

					baseAssets.push({
						route: route.replace(/^\//, ''),
						path: new URL(routeMeta.file),
					});
				}
			},
			'astro:build:done': async ({ dir }) => {
				const outDir = fileURLToPath(dir);
				for await (const path of assetPaths(outDir)) {
					baseAssets.push({ route: path, path: new URL(path, dir) });
				}

				const assets = await Promise.all(
					baseAssets.map(
						async ({ route, path: rawPath }): Promise<Asset> => {
							const file = Bun.file(rawPath);
							const asset = {
								route,
								file: path.relative(
									outDir,
									fileURLToPath(rawPath),
								),
								mime: file.type,
								gzip: false,
							};

							if (file.type.startsWith('text/')) {
								const compressed = Bun.gzipSync(
									await file.bytes(),
								);

								if (compressed.byteLength < file.size) {
									const compressedPath = `${fileURLToPath(rawPath)}.gz`;
									await Bun.write(compressedPath, compressed);

									asset.gzip = true;
									asset.file += '.gz';
								}
							}

							return asset;
						},
					),
				);

				await Bun.write(
					path.join(outDir, 'assets.json'),
					JSON.stringify(assets),
				);
			},
		},
	};
}

// biome-ignore lint/style/noDefaultExport:
export default config;
