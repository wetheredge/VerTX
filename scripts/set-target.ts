#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';
import { getFeatures } from './build-target.ts';
import { setRustAnayzerFeatures } from './set-ra-features.ts';
import type { Target } from './target-schema.ts';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

const targets = new Glob('*.toml').scanSync({ cwd: targetsDir });
const choices = Array.from(targets)
	.toSorted()
	.map((path) => ({ value: path.replace(/\.\w+$/, '') }));
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

await Promise.all([
	Bun.write(
		targetEnvFile,
		Object.entries(env)
			.map(([key, value]) => `${key}=${value}`)
			.join('\n'),
	),
	setRustAnayzerFeatures(features),
]);
