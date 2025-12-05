import * as fs from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// NOTE: update this if this file ever gets moved out of (or deeper within) /scripts/utils
export const repoRoot = fileURLToPath(new URL('../..', import.meta.url));
export const baseOutDir = join(repoRoot, 'out');

export async function fsReplaceSymlink(target: string, path: string) {
	try {
		await fs.lstat(path);
		await fs.rm(path);

		// biome-ignore lint/suspicious/noExplicitAny: there doesn't seem to be a specific Error subclass to use with an instanceof check
	} catch (e: any) {
		if (!('code' in e && e.code === 'ENOENT')) {
			throw e;
		}
	}

	await fs.symlink(target, path);
}
