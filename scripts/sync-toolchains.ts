#!/usr/bin/env bun

import { chdir } from 'node:process';
import { getRepoRoot, getXtensaToolchainVersion } from './utils';

// Consistent paths
chdir(await getRepoRoot());

const version = await getXtensaToolchainVersion();

await updateFile('.devcontainer/Dockerfile', (line) =>
	line.startsWith('FROM docker.io/espressif/idf-rust:')
		? `${line.split(':')[0]}:esp32s3_${version}`
		: line,
);

async function updateFile(path: string, map: (line: string) => string) {
	const file = Bun.file(path);
	const contents = await file.text();
	const updated = contents.split('\n').map(map).join('\n');
	await Bun.write(file, updated);
}
