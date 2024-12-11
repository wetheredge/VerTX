#!/usr/bin/env bun

import { exit } from 'node:process';
import Ajv, { type SomeJTDSchemaType } from 'ajv/dist/jtd';
import { Glob } from 'bun';
import { getRepoRoot } from './utils';

const schema: SomeJTDSchemaType = {
	definitions: {
		pin: { type: 'uint8' },
	},
	properties: {
		chip: { type: 'string' },
		pins: {
			properties: {
				mode: { ref: 'pin' },
				leds: { ref: 'pin' },
				analog: { elements: { ref: 'pin' } },
				switches: { elements: { ref: 'pin' } },
				ui: {
					properties: Object.fromEntries(
						['up', 'down', 'right', 'left'].map((name) => {
							return [name, { ref: 'pin' }];
						}),
					),
				},
				display: {
					discriminator: 'type',
					mapping: {
						ssd1306: {
							properties: {
								sda: { ref: 'pin' },
								scl: { ref: 'pin' },
							},
						},
					},
				},
			},
			optionalProperties: {
				backpack: {
					properties: {
						tx: { ref: 'pin' },
						rx: { ref: 'pin' },
					},
				},
			},
		},
	},
};

const ajv = new Ajv();
const validate = ajv.compile(schema);

const root = await getRepoRoot();
const targets = new Glob('targets/*.toml').scan({ cwd: root });
let allValid = true;
for await (const path of targets) {
	const absolutePath = `${root}/${path}`;
	const data = await import(absolutePath);
	if (!validate(data.default)) {
		allValid = false;
		console.log(path);
		for (const error of validate.errors ?? []) {
			console.log(
				`  ${error.instancePath} ${error.message ?? 'unknown error'}`,
			);
		}
	}
}

if (!allValid) {
	exit(1);
}
