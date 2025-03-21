#!/usr/bin/env bun

import { exit } from 'node:process';
import { Glob } from 'bun';
import { z } from 'zod';
import { repoRoot } from './utils';

const pin = z.number().nonnegative().int();
const schema = z
	.strictObject({
		chip: z.string(),
		pins: z.strictObject({
			leds: pin,
			analog: z.array(pin),
			switches: z.array(pin),
			ui: z.strictObject({
				up: pin,
				down: pin,
				left: pin,
				right: pin,
			}),
			display: z.discriminatedUnion('type', [
				z.strictObject({
					type: z.literal('ssd1306'),
					sda: pin,
					scl: pin,
				}),
			]),
		}),
	})
	.readonly();

const targets = new Glob('targets/*.toml').scan({ cwd: repoRoot });
let allValid = true;
for await (const path of targets) {
	const absolutePath = `${repoRoot}/${path}`;
	const data = await import(absolutePath);
	const result = schema.safeParse(data.default);
	if (!result.success) {
		allValid = false;
		console.log(path);
		const issues = result.error.issues.map((i) => ({
			...i,
			path: i.path
				.map((k) => (typeof k === 'string' ? `.${k}` : `[${k}]`))
				.join(''),
		}));
		const maxPathLength = Math.max(...issues.map((i) => i.path.length));
		for (const { path, message } of issues) {
			const padding = ' '.repeat(maxPathLength - path.length);
			console.log(`  ${path}: ${padding}${message}`);
		}
	}
}

if (!allValid) {
	exit(1);
}
