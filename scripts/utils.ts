import { exit } from 'node:process';
import { $ } from 'bun';

export const getRepoRoot = () =>
	$`git rev-parse --show-toplevel`.text().then((s) => s.trim());

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

export function panic(message: string) {
	console.error(message);
	if (import.meta.env.CI) {
		// biome-ignore lint/suspicious/noConsoleLog:
		console.log(`::error::${message}\n`);
	}
	exit(1);
}

export async function getXtensaToolchainVersion(): Promise<string> {
	const version = await Bun.file(
		new URL('../.config/xtensa-toolchain', import.meta.url),
	).text();
	return version.trim();
}
