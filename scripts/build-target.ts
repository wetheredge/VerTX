#!/usr/bin/env bun

import * as fs from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';
import { $, fileURLToPath } from 'bun';
import * as chip2Target from '../.config/chips.json';
import { schema, type Target } from './target-schema.ts';
import {
	baseOutDir,
	fsReplaceSymlink,
	isMain,
	orExit,
	panic,
	repoRoot,
} from './utils.ts';

export async function build(
	command: string,
	targetName: string,
	rawTarget: unknown,
	release?: boolean,
) {
	const target = schema.parse(rawTarget);
	const chip = getChipInfo(target.chip);

	const features = getFeatures(target).join(' ');

	const rustflags = [
		process.env.RUSTFLAGS,
		chip.cpu && `-Ctarget-cpu=${chip.cpu}`,
	]
		.filter((s) => s && s.length > 0)
		.join(' ');

	await orExit(
		$`cargo ${command} -p vertx -Zbuild-std=alloc,core --target ${chip.target} -F '${features}' ${release ? '--release' : ''}`
			.nothrow()
			.env({
				CARGO_TERM_COLOR: 'always',
				...process.env,
				RUSTFLAGS: rustflags,
				VERTX_TARGET: targetName,
			}),
	);

	if (command === 'build') {
		const profile = release ? 'release' : 'debug';

		const bin = join(repoRoot, 'target', chip.target, profile, 'vertx');
		const outDir = join(baseOutDir, 'firmware');
		fs.mkdirSync(outDir, { recursive: true });

		const outFile = `vertx_${targetName}_${profile}`;
		const outPath = join(outDir, outFile);
		fs.copyFileSync(bin, outPath, fs.constants.COPYFILE_FICLONE);

		await fsReplaceSymlink(outFile, join(outDir, 'vertx'));
	}
}

export function getFeatures(target: Target): Array<string> {
	return [`chip-${target.chip}`, `display-${target.display.type}`];
}

type ChipInfo = { target: string; cpu?: string };
function getChipInfo(chip: string): ChipInfo {
	const info = (chip2Target as Record<string, string | ChipInfo>)[chip];
	if (info == null) {
		throw new Error(`Missing info for chip '${chip}'`);
	}
	return typeof info === 'string' ? { target: info } : info;
}

if (isMain(import.meta.url)) {
	const usage = `usage: scripts/${import.meta.file} [--command=build/clippy/…] --target=<target> -- [...args]`;

	const { values } = parseArgs({
		args: Bun.argv.slice(2),
		options: {
			help: { short: 'h', type: 'boolean' },
			target: { type: 'string' },
			command: { type: 'string', default: 'build' },
			release: { short: 'r', type: 'boolean' },
		},
	});

	const targetName = values.target;
	if (targetName == null) {
		panic(`${usage}\n\nMissing --target`);
	}
	if (targetName === '') {
		panic('Missing value for --target. Try running `task :set-target`');
	}

	const targetPath = fileURLToPath(
		new URL(`../targets/${targetName}.toml`, import.meta.url),
	);

	if (!fs.existsSync(targetPath)) {
		panic(`Cannot find target '${targetName}'`);
	}

	const target = await import(targetPath);
	await build(values.command, targetName, target.default, values.release);
}
