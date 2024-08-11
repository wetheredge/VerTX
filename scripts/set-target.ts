#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

const CHIP_TO_RUST: Record<string, [toolchain: string, target: string]> = {
	esp32s3: ['esp', 'xtensa-esp32s3-none-elf'],
};

type Target = {
	chip: string;
	server: string;
	pins: Record<string, unknown>;
};

const targets = new Glob('*.toml').scanSync({ cwd: targetsDir });
const choices = Array.from(targets)
	.toSorted()
	.map((path) => {
		const name = path.replace(/\.\w+$/, '');
		return {
			name,
			value: [`${targetsDir}/${path}`, name],
		};
	});

const [path, name] = await select({ message: 'Choose a target:', choices });
const target: Target = await import(path);

const env: Record<string, string> = {
	VERTX_TARGET: name,
	VERTX_CHIP: target.chip,
	VERTX_RUST_TOOLCHAIN: CHIP_TO_RUST[target.chip][0],
	VERTX_RUST_TARGET: CHIP_TO_RUST[target.chip][1],
};

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}="${value}"`)
		.join('\n'),
);
