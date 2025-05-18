import { exit } from 'node:process';
import { fileURLToPath } from 'bun';

// NOTE: update this if this file ever gets moved out of (or deeper within) /scripts/
export const repoRoot = fileURLToPath(new URL('..', import.meta.url));

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

export function panic(message: string) {
	console.error(message);
	if (import.meta.env.CI) {
		// biome-ignore lint/suspicious/noConsoleLog:
		console.log(`::error::${message}\n`);
	}
	exit(1);
}
