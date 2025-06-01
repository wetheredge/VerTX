#!/usr/bin/env bun

import { select } from '@inquirer/prompts';
import { Glob } from 'bun';
import { getFeatures } from './build-target.ts';
import { setRustAnayzerFeatures } from './set-ra-features.ts';
import { type Target, schema } from './target-schema.ts';
import { repoRoot } from './utils.ts';

const [name, target] = await choose(`${repoRoot}/targets`);

await Promise.all([
	writeEnv(name, target, `${repoRoot}/.env.target`),
	setRustAnayzerFeatures(getFeatures(target)),
]);

async function choose(targetsDir: string): Promise<[string, Target]> {
	const targets = new Glob('*.toml').scanSync({ cwd: targetsDir });
	const choices = Array.from(targets)
		.toSorted()
		.map((path) => ({ value: path.replace(/\.\w+$/, '') }));

	const name = await select({
		message: 'Choose a target:',
		choices,
		default: import.meta.env.VERTX_TARGET,
	});

	const target = await import(`${targetsDir}/${name}.toml`);
	return [name, schema.parse(target.default)];
}

async function writeEnv(name: string, target: Target, out: string) {
	const env: Record<string, string> = {
		VERTX_TARGET: name,
		VERTX_CHIP: target.chip,
	};

	await Bun.write(
		out,
		Object.entries(env)
			.map(([key, value]) => `${key}=${value}`)
			.join('\n'),
	);
}
