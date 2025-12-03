#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { argv, exit, stdin, stdout } from 'node:process';

const direction = argv[2];

if (direction == null || !['up', 'down'].includes(direction)) {
	console.info('Usage: migrate.ts [up|down] < config.in > config.out');
	exit(1);
}

const migrateFile = readFileSync(`../target/migrate-${direction}.wasm`);
const { instance: migrate } = await WebAssembly.instantiate(migrateFile);
const memory = migrate.exports.memory as WebAssembly.Memory;
const dataOffset = migrate.exports.DATA as WebAssembly.Global;

const memoryView = new Uint8Array(memory.buffer, dataOffset.value);
const run = migrate.exports.run as () => number;

memoryView.set(readFileSync(stdin.fd));

let length: number;
try {
	length = run();
} catch {
	console.error(
		'Failed to migrate config. Perhaps it is invalid or not the version expected by the migration?',
	);
	exit(1);
}

writeFileSync(stdout.fd, memoryView.subarray(0, length));
