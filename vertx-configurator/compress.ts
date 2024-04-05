import { resolveConfig } from 'vite';

const viteConfig = await resolveConfig({}, 'build');
const { outDir } = viteConfig.build;

const assets: Array<{
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
}> = [];

for await (const route of new Bun.Glob('**').scan({ dot: true, cwd: outDir })) {
	const rawPath = `${outDir}/${route}`;
	const compressedPath = `${rawPath}.gz`;

	const raw = Bun.file(rawPath);
	const compressed = Bun.gzipSync(await raw.arrayBuffer());

	const useGzip = compressed.byteLength < raw.size;
	if (useGzip) {
		await Bun.write(compressedPath, compressed);
	}

	const asset = {
		route: `/${route.replace(/(index)?\.html$/, '')}`,
		file: useGzip ? `${route}.gz` : route,
		mime: raw.type,
		gzip: useGzip,
	};

	assets.push(asset);
}

await Bun.write(`${outDir}/assets.json`, JSON.stringify(assets));
