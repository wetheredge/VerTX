import { exit } from 'node:process';
import { fileURLToPath } from 'bun';

// NOTE: update this if this file ever gets moved out of (or deeper within) /scripts/
export const repoRoot = fileURLToPath(new URL('..', import.meta.url));

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

export function humanBytes(bytes: number): string {
	if (bytes < 1024) {
		return `${bytes} B`;
	}

	const prefixes = ['K', 'M', 'G', 'T'];

	let size = bytes;
	let prefix = -1;
	do {
		size /= 1024;
		prefix++;
	} while (size >= 1024 && prefix + 1 < prefixes.length);

	return `${size.toFixed(2)} ${prefixes[prefix]}iB`;
}

export function isMain(importMetaUrl: string): boolean {
	return process.argv[1] === fileURLToPath(importMetaUrl);
}

export async function orExit(cmd: Bun.$.ShellPromise) {
	const result = await cmd.nothrow();
	if (result.exitCode !== 0) {
		exit(result.exitCode);
	}
}

export function panic(message: string) {
	console.error(message);
	if (import.meta.env.CI) {
		// biome-ignore lint/suspicious/noConsoleLog:
		console.log(`::error::${message}\n`);
	}
	exit(1);
}
