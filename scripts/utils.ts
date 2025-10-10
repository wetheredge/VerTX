import * as fs from 'node:fs/promises';
import { join } from 'node:path';
import { argv, exit } from 'node:process';
import { fileURLToPath } from 'bun';

// NOTE: update this if this file ever gets moved out of (or deeper within) /scripts/
export const repoRoot = fileURLToPath(new URL('..', import.meta.url));
export const baseOutDir = join(repoRoot, 'out');

export async function fileAppend(
	path: string | URL,
	toAppend: string,
): Promise<void> {
	const file = Bun.file(path);
	const writer = file.writer();
	// FIXME: there really isn't a way to seek to the end?
	writer.write(await file.bytes());
	writer.write(toAppend);
	await writer.end();
}

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

export async function orExit(cmd: Bun.$.ShellPromise) {
	const result = await cmd.nothrow();
	if (result.exitCode !== 0) {
		exit(result.exitCode);
	}
}

export function humanBytes(bytes: number): string {
	const base = 1024;

	if (bytes < base) {
		return `${bytes} B`;
	}

	const prefixes = ['K', 'M', 'G', 'T'];

	let size = bytes;
	let prefix = -1;
	do {
		size /= base;
		prefix++;
	} while (size >= base && prefix + 1 < prefixes.length);

	return `${size.toFixed(2)} ${prefixes[prefix]}iB`;
}

export function isMain(importMetaUrl: string): boolean {
	return argv[1] === fileURLToPath(importMetaUrl);
}

export function panic(message: string): never {
	console.error(message);
	if (import.meta.env.CI) {
		console.log(`::error::${message}\n`);
	}
	exit(1);
}
