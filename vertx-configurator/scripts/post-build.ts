#!/usr/bin/env bun

import { assetPaths, outDir, pathToRoute, prettySize } from './utils';

type Asset = {
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
};

const assets = new Array<Asset>();
let totalSize = 0;
for await (const path of assetPaths()) {
	const rawPath = `${outDir}/${path}`;
	const compressedPath = `${rawPath}.gz`;

	const raw = Bun.file(rawPath);
	const asset: Asset = {
		route: pathToRoute(path),
		file: path,
		mime: raw.type,
		gzip: false,
	};

	let size = raw.size;
	if (raw.type.startsWith('text/')) {
		const compressed = Bun.gzipSync(await raw.bytes());

		if (compressed.byteLength < size) {
			size = await Bun.write(compressedPath, compressed);
			asset.gzip = true;
			asset.file = `${path}.gz`;
		}
	}

	totalSize += size;
	assets.push(asset);
}

const strip = (key: string, value: unknown) =>
	key === 'size' || key === 'raw' ? undefined : value;
await Bun.write(`${outDir}/assets.json`, JSON.stringify(assets, strip));

console.info('Total size:', prettySize(totalSize));
