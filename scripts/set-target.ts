#!/usr/bin/env bun

import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';
import { getFeatures } from './build-target.ts';
import { setRustAnayzerFeatures } from './set-ra-features.ts';
import type { Target } from './target-schema.ts';
import { baseOutDir, repoRoot } from './utils.ts';

const targetsDir = join(repoRoot, 'targets');
const envFile = join(baseOutDir, 'target');

const targets = new Glob('*.toml').scanSync({ cwd: targetsDir });
const fileExtension = /\.\w+$/;
const choices = Array.from(targets)
	.toSorted()
	.map((path) => ({ value: path.replace(fileExtension, '') }));
const targetName = await select({
	message: 'Choose a target:',
	choices,
	default: import.meta.env.VERTX_TARGET,
});
const target: Target = await import(`${targetsDir}/${targetName}.toml`);

const features = getFeatures(target);
const env: Record<string, string> = {
	VERTX_TARGET: targetName,
	VERTX_CHIP: target.chip,
};

if (!existsSync(baseOutDir)) {
	mkdirSync(baseOutDir);
}

await Promise.all([
	Bun.write(
		envFile,
		Object.entries(env)
			.map(([key, value]) => `${key}=${value}`)
			.join('\n'),
	),
	setRustAnayzerFeatures(features),
]);
