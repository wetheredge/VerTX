import { argv, exit } from 'node:process';
import { fileURLToPath } from 'bun';

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

export async function orExit(cmd: Bun.$.ShellPromise) {
	const result = await cmd.nothrow();
	if (result.exitCode !== 0) {
		exit(result.exitCode);
	}
}
