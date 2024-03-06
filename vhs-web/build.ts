import { createReadStream, createWriteStream } from 'node:fs';
import { rm, writeFile } from 'node:fs/promises';
import { exit } from 'node:process';
import { pipeline } from 'node:stream/promises';
import { createGzip } from 'node:zlib';
import { resolveConfig } from 'vite';
import { build } from 'vite';

const { outDir } = await resolveConfig({}, 'build').then(
	(config) => config.build,
);

let outputs = await build();
if ('output' in outputs) {
	outputs = [outputs];
} else if (!Array.isArray(outputs)) {
	throw new Error('Unexpected RollupWatcher');
}

const files = outputs
	.flatMap((output) => output.output)
	.map((output) => output.fileName);

const assets: Array<{
	route: string;
	file: string;
	mime: string;
	gzip: boolean;
}> = [];

const mimes: Record<string, string> = {
	css: 'text/css',
	html: 'text/html; charset=UTF-8',
	js: 'text/javascript',
	svg: 'image/svg+xml',
};

for (const file of files) {
	const rawPath = `${outDir}/${file}`;
	const compressedPath = `${rawPath}.gz`;

	const reader = createReadStream(rawPath);
	const gzip = createGzip();
	const writer = createWriteStream(compressedPath);

	await pipeline(reader, gzip, writer);

	const useGzip = writer.bytesWritten < reader.bytesRead;

	writer.close();
	gzip.close();
	reader.close();

	const mime = mimes[file.split('.').at(-1)];
	if (!mime) {
		throw new Error(`unknown mime: '${file}'`);
	}

	const finalFile = useGzip ? `${file}.gz` : file;
	await rm(useGzip ? rawPath : compressedPath);

	const asset = {
		route: `/${file.replace(/(index)?\.html$/, '')}`,
		file: finalFile,
		mime,
		gzip: useGzip,
	};

	assets.push(asset);
}

await writeFile(`${outDir}/assets.json`, JSON.stringify(assets));
exit(0);
