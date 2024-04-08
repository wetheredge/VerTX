import { resolveConfig } from 'vite';

const viteConfig = await resolveConfig({}, 'build');
const { outDir } = viteConfig.build;

const assets: Array<{
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
}> = [];

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
	assets.push(asset);
}

await Bun.write(`${outDir}/assets.json`, JSON.stringify(assets));

console.info(`Total size: ${(totalSize / 1024).toFixed(2)}KiB`);
