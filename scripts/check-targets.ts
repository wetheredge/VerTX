#!/usr/bin/env bun

import { exit } from 'node:process';
import { Glob } from 'bun';
import { schema } from './target-schema';
import { repoRoot } from './utils';

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
