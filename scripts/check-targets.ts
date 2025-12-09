#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { exit } from 'node:process';
import * as toml from 'smol-toml';
import { glob } from 'tinyglobby';
import { repoRoot } from '#utils/fs';
import { schema } from '#utils/target';

const targets = await glob('*.toml', {
	cwd: join(repoRoot, 'targets'),
	absolute: true,
});

const results = await Promise.all(targets.map(checkTarget));
let allValid = true;
for (const [target, issues] of results.filter((x) => x != null)) {
	allValid = false;
	console.log(target);
	const maxPathLength = Math.max(...issues.map((i) => i.path.length));
	for (const { path, message } of issues) {
		const padding = ' '.repeat(maxPathLength - path.length);
		console.log(`  ${path}: ${padding}${message}`);
	}
}

if (!allValid) {
	exit(1);
}

type TargetIssue = { message: string; path: string };
type CheckResult = undefined | [file: string, Array<TargetIssue>];
async function checkTarget(path: string): Promise<CheckResult> {
	const data = toml.parse(await readFile(path, { encoding: 'utf8' }));
	const result = await schema.safeParseAsync(data);
	if (result.success) {
		return;
	}

	const issues = result.error.issues.map(({ path, message }) => ({
		message,
		path: path
			.map((k) => (typeof k === 'string' ? `.${k}` : `[${String(k)}]`))
			.join(''),
	}));

	return [basename(path), issues];
}
