#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

const BACKPACK_CHIPS = ['esp32', 'esp32c3', 'esp32s3'];

type Target = {
	chip: string;
	server: string;
	pins: Record<string, unknown>;
};

const targets = new Glob('*.toml').scanSync({ cwd: targetsDir });
const choices = Array.from(targets)
	.toSorted()
	.map((path) => ({ value: path.replace(/\.\w+$/, '') }));
const targetName = await select({ message: 'Choose a target:', choices });
const target: Target = await import(`${targetsDir}/${targetName}.toml`);

const env: Record<string, string> = {
	VERTX_TARGET: targetName,
	VERTX_CHIP: target.chip,
};

if ('backpack' in target.pins) {
	const choices = BACKPACK_CHIPS.map((name) => ({ name, value: name }));
	const chip = await select({ message: 'Choose a backpack chip:', choices });
	env.VERTX_BACKPACK_CHIP = chip;
}

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}=${value}`)
		.join('\n'),
);
