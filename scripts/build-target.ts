#!/usr/bin/env node

import * as fs from 'node:fs';
import { basename, join } from 'node:path';
import { argv, env } from 'node:process';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { cargoBuild } from '#utils/cargo';
import { isMain, panic } from '#utils/cli';
import { baseOutDir, fsReplaceSymlink, repoRoot } from '#utils/fs';
import { loadTarget, type Target } from '#utils/target';
import chip2Target from '../.config/chips.json' with { type: 'json' };

export async function build(
	command: string,
	targetName: string,
	release?: boolean,
	extraArgs: Array<string> = [],
) {
	const target = await loadTarget(targetName);
	const chip = getChipInfo(target.chip);

	const rustflags = [env.RUSTFLAGS, chip.cpu && `-Ctarget-cpu=${chip.cpu}`]
		.filter((s) => s && s.length > 0)
		.join(' ');

	const path = [join(repoRoot, '.tools/gcc/bin'), env.PATH].join(':');

	await cargoBuild({
		command,
		buildStd: 'alloc,core',
		target: chip.target,
		features: getFeatures(target),
		release,
		extraArgs,
		env: {
			...env,
			PATH: path,
			RUSTFLAGS: rustflags,
			VERTX_TARGET: targetName,
		},
	});

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

export type ChipInfo = { target: string; cpu?: string };
export function getChipInfo(chip: string): ChipInfo {
	const info = (chip2Target as Record<string, string | ChipInfo>)[chip];
	if (info == null) {
		throw new Error(`Missing info for chip '${chip}'`);
	}
	return typeof info === 'string' ? { target: info } : info;
}

if (isMain(import.meta.url)) {
	const usage = `usage: scripts/${basename(import.meta.filename)} [--command=build/clippy/…] --target=<target> -- [...args]`;

	const { values, positionals } = parseArgs({
		args: argv.slice(2),
		options: {
			help: { short: 'h', type: 'boolean' },
			target: { type: 'string' },
			command: { type: 'string', default: 'build' },
			release: { short: 'r', type: 'boolean' },
		},
		allowPositionals: true,
	});

	const targetName = values.target;
	if (targetName == null) {
		panic(`${usage}\n\nMissing --target`);
	}
	if (targetName === '') {
		panic('Missing value for --target. Try running `wrun /target`');
	}

	const targetPath = fileURLToPath(
		new URL(`../targets/${targetName}.toml`, import.meta.url),
	);

	if (!fs.existsSync(targetPath)) {
		panic(`Cannot find target '${targetName}'`);
	}

	await build(values.command, targetName, values.release, positionals);
}
