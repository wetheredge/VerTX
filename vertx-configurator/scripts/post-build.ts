#!/usr/bin/env bun

import { fileURLToPath } from 'bun';

const outDir = fileURLToPath(new URL('../dist', import.meta.url));

type Asset = {
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
};

type Segment = { content: string } | { placeholder: string };

type Route = {
	route: string;
	segments: Array<Segment>;
};

const manifest = {
	assets: new Array<Asset>(),
	routes: new Array<Route>(),
};

let totalSize = 0;
const routePaths = new Array<string>();

for await (const path of new Bun.Glob('**').scan(outDir)) {
	if (path.endsWith('.html')) {
		routePaths.push(path);
		continue;
	}

	const rawPath = `${outDir}/${path}`;
	const compressedPath = `${rawPath}.gz`;

	const raw = Bun.file(rawPath);
	const asset: Asset = {
		route: path,
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
	manifest.assets.push(asset);
}

for (const path of routePaths) {
	const file = Bun.file(`${outDir}/${path}`);
	totalSize += (await file.bytes()).byteLength;

	const contents = await file.text();
	const segments = contents
		.split(/\$([a-z-]+)\$/)
		.map((segment, i) =>
			i % 2 ? { placeholder: segment } : { content: segment },
		);

	manifest.routes.push({
		route: path.replace(/\.html$/, ''),
		segments,
	});
}

await Bun.write(`${outDir}/manifest.json`, JSON.stringify(manifest));

console.info(`Total size: ${(totalSize / 1024).toFixed(2)}KiB`);
