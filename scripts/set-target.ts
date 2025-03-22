#!/usr/bin/env bun

import { cwd } from 'node:process';
import { select } from '@inquirer/prompts';
import { Glob } from 'bun';
import * as chip2target from '../.config/chip2target.json';

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
	VERTX_RUSTC_TARGET: getRustcTarget(target.chip),
	VERTX_FEATURES: `display-${target.pins.display.type}`,
};

await Bun.write(
	targetEnvFile,
	Object.entries(env)
		.map(([key, value]) => `${key}=${value}`)
		.join('\n'),
);

function getRustcTarget(chip: string): string {
	if (chip in chip2target) {
		// @ts-expect-error
		return chip2target[chip];
	}

	throw new Error(`Missing rustc target for chip '${chip}'`);
}
