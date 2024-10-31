import { exit } from 'node:process';
import { $ } from 'bun';

export const getRepoRoot = () =>
	$`git rev-parse --show-toplevel`.text().then((s) => s.trim());

export function panic(message: string) {
	console.error(message);
	if (import.meta.env.CI) {
		// biome-ignore lint/suspicious/noConsoleLog:
		console.log(`::error::${message}\n`);
	}
	exit(1);
}
