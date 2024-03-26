import { createReadStream, createWriteStream } from 'node:fs';
import { writeFile } from 'node:fs/promises';
import { pipeline } from 'node:stream/promises';
import { createGzip } from 'node:zlib';
import glob from 'fast-glob';
import { resolveConfig } from 'vite';

const mimes: Record<string, string> = {
	css: 'text/css',
	html: 'text/html; charset=UTF-8',
	js: 'text/javascript',
	svg: 'image/svg+xml',
};

const viteConfig = await resolveConfig({}, 'build');
const { outDir } = viteConfig.build;

const assets: Array<{
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
}> = [];

for (const route of glob.sync('**', { dot: true, cwd: outDir })) {
	const rawPath = `${outDir}/${route}`;
	const compressedPath = `${rawPath}.gz`;

	const reader = createReadStream(rawPath);
	const gzip = createGzip();
	const writer = createWriteStream(compressedPath);

	await pipeline(reader, gzip, writer);

	const useGzip = writer.bytesWritten < reader.bytesRead;

	writer.close();
	gzip.close();
	reader.close();

	const mime = mimes[route.split('.').at(-1)];
	if (!mime) {
		throw new Error(`unknown mime: '${route}'`);
	}

	const asset = {
		route: `/${route.replace(/(index)?\.html$/, '')}`,
		file: useGzip ? `${route}.gz` : route,
		mime,
		gzip: useGzip,
	};

	assets.push(asset);
}

await writeFile(`${outDir}/assets.json`, JSON.stringify(assets));
