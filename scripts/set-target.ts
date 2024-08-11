#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

const NIGHTLY = 'nightly-2024-06-08';
const CHIP_TO_RUST: Record<string, [toolchain: string, target: string]> = {
	esp32: ['esp', 'xtensa-esp32-none-elf'],
	esp32c3: [NIGHTLY, 'xtensa-esp32c3-none-elf'],
	esp32s3: ['esp', 'xtensa-esp32s3-none-elf'],
	rp2040: [NIGHTLY, 'thumbv6m-none-eabi'],
	stm32f407: [NIGHTLY, 'thumbv7em-none-eabihf'],
};

const BACKPACK_CHIPS = ['esp32', 'esp32c3', 'esp32s3'];

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

const env: Record<string, string> = { VERTX_TARGET: name };

const chipEnv = (chip: string, prefix: string) => {
	env[`${prefix}_CHIP`] = chip;
	env[`${prefix}_RUST_TOOLCHAIN`] = CHIP_TO_RUST[chip][0];
	env[`${prefix}_RUST_TARGET`] = CHIP_TO_RUST[chip][1];
};

chipEnv(target.chip, 'VERTX');

if ('backpack' in target.pins) {
	const choices = BACKPACK_CHIPS.map((name) => ({ name, value: name }));

	const chip = await select({ message: 'Choose a backpack chip:', choices });
	chipEnv(chip, 'VERTX_BACKPACK');
}

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}="${value}"`)
		.join('\n'),
);
