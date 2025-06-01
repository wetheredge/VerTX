#!/usr/bin/env bun

import { exit } from 'node:process';

const direction = Bun.argv[2];

if (direction == null || !['up', 'down'].includes(direction)) {
	console.info('Usage: migrate.ts [up|down] < config.in > config.out');
	exit(1);
}

const migrateFile = await Bun.file(
	`../target/migrate-${direction}.wasm`,
).bytes();
const { instance: migrate } = await WebAssembly.instantiate(migrateFile);
const memory = migrate.exports.memory as WebAssembly.Memory;
const dataOffset = migrate.exports.DATA as WebAssembly.Global;

const memoryView = new Uint8Array(memory.buffer, dataOffset.value);
const run = migrate.exports.run as () => number;

const input = await Bun.stdin.bytes();
memoryView.set(input);
let length: number;
try {
	length = run();
} catch {
	console.error(
		'Failed to migrate config. Perhaps it is invalid or not the version expected by the migration?',
	);
	exit(1);
}

const output = memoryView.slice(0, length);
await Bun.write(Bun.stdout, output);
