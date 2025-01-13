#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';

const targetsDir = `${cwd()}/targets`;
const targetEnvFile = '.env.target';

const BACKPACK_CHIPS = ['esp32', 'esp32c3', 'esp32s3'];
const RUSTC_TARGETS: Record<string, string> = {
	esp32: 'xtensa-esp32-none-elf',
	esp32c3: 'riscv32imc-unknown-none-elf',
	esp32s3: 'xtensa-esp32s3-none-elf',
	rp2040: 'thumbv6m-none-eabi',
	stm32f407: 'thumbv7em-none-eabihf',
};

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
	VERTX_RUSTC_TARGET: getRustcTarget(target.chip),
	VERTX_FEATURES: `display-${target.pins.display.type}`,
};

if ('backpack' in target.pins) {
	const choices = BACKPACK_CHIPS.map((name) => ({ name, value: name }));
	const chip = await select({
		message: 'Choose a backpack chip:',
		choices,
		default: import.meta.env.VERTX_BACKPACK_CHIP,
	});
	env.VERTX_BACKPACK_CHIP = chip;
	env.VERTX_BACKPACK_RUSTC_TARGET = getRustcTarget(chip);
}

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}=${value}`)
		.join('\n'),
);

function getRustcTarget(chip: string): string {
	if (chip in RUSTC_TARGETS) {
		return RUSTC_TARGETS[chip];
	}

	throw new Error(`Missing rustc target for chip '${chip}'`);
}
