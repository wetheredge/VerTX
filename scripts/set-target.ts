#!/usr/bin/env node

import { existsSync, mkdirSync } from 'node:fs';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import process from 'node:process';
import { select } from '@inquirer/prompts';
import { glob } from 'tinyglobby';
import { baseOutDir, repoRoot } from '#utils/fs';
import { getChipInfo, getFeatures } from './build-target.ts';
import { setRustAnayzerConfig } from './set-ra-config.ts';
import type { Target } from './target-schema.ts';

const targetsDir = join(repoRoot, 'targets');
const envFile = join(baseOutDir, 'target');

const targets = await glob('*.toml', { cwd: join(repoRoot, 'targets') });
const fileExtension = /\.\w+$/;
const choices = targets
	.toSorted()
	.map((path) => ({ value: path.replace(fileExtension, '') }));
const targetName = await select({
	message: 'Choose a target:',
	choices,
	default: process.env.VERTX_TARGET,
});
const target: Target = await import(`${targetsDir}/${targetName}.toml`);

const features = getFeatures(target);
const triple = getChipInfo(target.chip).target;
const env: Record<string, string> = {
	VERTX_TARGET: targetName,
	VERTX_CHIP: target.chip,
};

if (!existsSync(baseOutDir)) {
	mkdirSync(baseOutDir);
}

await Promise.all([
	writeFile(
		envFile,
		Object.entries(env)
			.map(([key, value]) => `${key}=${value}`)
			.join('\n'),
	),
	setRustAnayzerConfig({ triple, features, env }),
]);
