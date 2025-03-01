#!/usr/bin/env bun

import { fileURLToPath } from 'bun';

const outDir = fileURLToPath(new URL('../dist', import.meta.url));

type Asset = {
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
};

const assets = new Array<Asset>();

let totalSize = 0;

for await (const path of new Bun.Glob('**').scan(outDir)) {
	const rawPath = `${outDir}/${path}`;
	const compressedPath = `${rawPath}.gz`;

	const raw = Bun.file(rawPath);
	const asset: Asset = {
		route: path.replace(/(^|\/)index\.html/, '').replace(/\.html$/, ''),
		file: path,
		mime: raw.type,
		gzip: false,
	};

	let size = raw.size;
	if (raw.type.startsWith('text/')) {
		const compressed = Bun.gzipSync(await raw.arrayBuffer());

		if (compressed.byteLength < size) {
			size = await Bun.write(compressedPath, compressed);
			asset.gzip = true;
			asset.file = `${path}.gz`;
		}
	}

	totalSize += size;
	assets.push(asset);
}

await Bun.write(`${outDir}/assets.json`, JSON.stringify(assets));

console.info(`Total size: ${(totalSize / 1024).toFixed(2)}KiB`);
