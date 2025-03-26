#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

type Pins = Record<string, unknown>;
type Target = {
	chip: string;
	pins: Pins & {
		display: { type: string } & Pins;
	};
};

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

const env: Record<string, string> = {
	VERTX_TARGET: targetName,
	VERTX_CHIP: target.chip,
};

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}=${value}`)
		.join('\n'),
);
