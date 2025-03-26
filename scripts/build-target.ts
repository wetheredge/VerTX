#!/usr/bin/env bun

import { existsSync, rmSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { exit } from 'node:process';
import { $, fileURLToPath } from 'bun';
import * as chip2Target from '../.config/chip2target.json';
import { schema } from './target-common';

export async function build(
	command: string,
	targetName: string,
	rawTarget: unknown,
	args: Array<string> = [],
) {
	const target = schema.parse(rawTarget);

	// @ts-expect-error
	const tuple: string | undefined = chip2Target[target.chip];
	if (!tuple) {
		throw new Error(`Missing target tuple for chip '${target.chip}'`);
	}

	const features = [
		`chip-${target.chip}`,
		`display-${target.pins.display.type}`,
	].join(' ');

	await $`cargo ${command} -p vertx -Zbuild-std=alloc,core --target ${tuple} -F '${features}' ${args}`.env(
		{
			CARGO_TERM_COLOR: 'always',
			...process.env,
			VERTX_TARGET: targetName,
		},
	);

	if (command === 'build') {
		const targetDir = fileURLToPath(new URL('../target', import.meta.url));
		const isRelease = args.includes('-r') || args.includes('--release');
		const profile = isRelease ? 'release' : 'debug';

		const from = join(tuple, profile, 'vertx');
		const to = join(targetDir, 'vertx');

		if (existsSync(to)) {
			rmSync(to);
		}

		symlinkSync(from, to);
	}
}

if (Bun.argv[1] === import.meta.path) {
	const usage = `usage: scripts/${import.meta.file} [--command build/clippy/…] <target> [...args]`;
	let args = Bun.argv.slice(2);

	if (args[0] === '--help' || args[0] === '-h') {
		console.info(usage);
		exit(0);
	}

	let command = 'build';
	if (args[0] === '--command') {
		command = args[1];
		args = args.slice(2);
	} else if (args[0]?.startsWith('--command=')) {
		command = args[0].split('=', 2)[1];
		args = args.slice(1);
	}

	if (args.length < 1) {
		console.error(usage);
		exit(1);
	}

	const [targetName, ...buildArgs] = args;
	const targetPath = fileURLToPath(
		new URL(`../targets/${targetName}.toml`, import.meta.url),
	);

	if (!existsSync(targetPath)) {
		console.error(`Cannot find target '${targetName}'`);
		exit(1);
	}

	const target = await import(targetPath);
	await build(command, targetName, target.default, buildArgs);
}
