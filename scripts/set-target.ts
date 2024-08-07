#!/usr/bin/env bun

import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = 'targets';
const targetEnvFile = '.env.target';

type Target = {
	hal: string;
};

const targets = new Glob('*.toml').scanSync({
	cwd: targetsDir,
	absolute: true,
});
const choices = Array.from(targets)
	.toSorted()
	.map((path) => {
		// biome-ignore lint/style/noNonNullAssertion: the absolute path to a target file will always contain a `/`
		const name = path
			.split('/')
			.at(-1)!
			.replace(/\.\w+$/, '');
		return {
			name,
			value: [path, name],
		};
	});

const [path, name] = await select({ message: 'Choose a target:', choices });
const target: Target = await import(path);

const targetEnv = Object.entries({
	TARGET: name,
	HAL: target.hal,
})
	.map(([key, value]) => `VERTX_${key}="${value}"`)
	.join('\n');
await Bun.write(targetEnvFile, targetEnv);
