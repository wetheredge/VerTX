import { fileURLToPath } from 'node:url';

export const isSimulator = process.env.VERTX_SIMULATOR === 'true';
export const outDir = fileURLToPath(
	new URL(
		`../../out/${isSimulator ? 'simulator/' : ''}configurator`,
		import.meta.url,
	),
);

export function assetPaths(): AsyncIterableIterator<string> {
	return new Bun.Glob('**').scan(outDir);
}

export function pathToRoute(path: string): string {
	return path.replace(/(^|\/)index\.html/, '').replace(/\.html$/, '');
}

export function prettySize(bytes: number): string {
	if (bytes < 1024) {
		return `${bytes} bytes`;
	}

	return `${(bytes / 1024).toFixed(2)}KiB`;
}
