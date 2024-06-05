#!/usr/bin/env bun

import { resolveConfig } from 'vite';

const viteConfig = await resolveConfig({}, 'build');
const { outDir } = viteConfig.build;

type Asset = {
	readonly route: string;
	readonly file: string;
	readonly mime: string;
	readonly gzip: boolean;
};

const manifest: { index?: Asset; assets: Array<Asset> } = {
	assets: [],
};

let totalSize = 0;
for await (const route of new Bun.Glob('**').scan({ dot: true, cwd: outDir })) {
	const rawPath = `${outDir}/${route}`;
	const compressedPath = `${rawPath}.gz`;

	const raw = Bun.file(rawPath);
	const asset = {
		route: `/${route.replace(/(index)?\.html$/, '')}`,
		file: route,
		mime: raw.type,
		gzip: false,
	};

	let size = raw.size;
	if (raw.type.startsWith('text/')) {
		const compressed = Bun.gzipSync(await raw.arrayBuffer());

		if (compressed.byteLength < size) {
			size = await Bun.write(compressedPath, compressed);
			asset.gzip = true;
			asset.file = `${route}.gz`;
		}
	}

	totalSize += size;
	if (asset.route === '/') {
		manifest.index = asset;
	} else {
		manifest.assets.push(asset);
	}
}

await Bun.write(`${outDir}/manifest.json`, JSON.stringify(manifest));

console.info(`Total size: ${(totalSize / 1024).toFixed(2)}KiB`);
