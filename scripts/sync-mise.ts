#!/usr/bin/env node

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import TOML from 'smol-toml';
import { panic } from '#utils/cli';
import { repoRoot } from '#utils/fs';
import versions from '../.config/versions.json' with { type: 'json' };

const tools = {
	actionlint: 'aqua:rhysd/actionlint',
	binaryen: 'aqua:WebAssembly/binaryen',
	biome: 'aqua:biomejs/biome',
	'cargo-nextest': 'cargo:cargo-nextest',
	'cargo-shear': 'cargo:cargo-shear',
	'cargo-sort': 'cargo:cargo-sort',
	dprint: 'aqua:dprint/dprint',
	nodejs: 'node',
	pnpm: 'pnpm',
	'probe-rs-tools': 'cargo:probe-rs-tools',
	typos: 'aqua:crate-ci/typos',
	'wasm-bindgen-cli': 'cargo:wasm-bindgen-cli',
	wrun: 'ubi:wetheredge/wrun',
};

const miseTools: Record<string, string> = {};
for (const [tool, miseName] of Object.entries(tools)) {
	if (!(tool in versions)) {
		panic(`Missing version for '${tool}'`);
	}
	// @ts-expect-error: TS infers the type of `versions` from the json file, then doesn't like dynamic index exprs
	miseTools[miseName] = versions[tool];
}

const miseConfig = {
	tools: miseTools,
};
const miseConfigFile = join(repoRoot, '.config/mise.toml');
writeFileSync(miseConfigFile, TOML.stringify(miseConfig));
