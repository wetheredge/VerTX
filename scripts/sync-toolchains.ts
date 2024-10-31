#!/usr/bin/env bun

import { chdir } from 'node:process';
import { chips, xtensa } from '../.config/toolchains.json';
import { getRepoRoot } from './utils';

// Consistent paths
chdir(await getRepoRoot());

const chipDataPrefix = '  raw_chip_data:';
await updateFile('.config/tasks/Taskfile.embedded.yaml', (line) =>
	line.startsWith(chipDataPrefix)
		? `${chipDataPrefix} '${JSON.stringify(chips)}'`
		: line,
);

await updateFile('README.md', (line) =>
	line.includes('espup install')
		? line.replace(
				/--toolchain-version\s+[^\s]+/,
				`--toolchain-version ${xtensa}`,
			)
		: line,
);

await updateFile('.devcontainer/Dockerfile', (line) =>
	line.startsWith('FROM docker.io/espressif/idf-rust:')
		? `${line.split(':')[0]}:esp32s3_${xtensa}`
		: line,
);

async function updateFile(path: string, map: (line: string) => string) {
	const file = Bun.file(path);
	const contents = await file.text();
	const updated = contents.split('\n').map(map).join('\n');
	await Bun.write(file, updated);
}
