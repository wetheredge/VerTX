#!/usr/bin/env bun

import { exists, mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { argv } from 'node:process';
import TOML from 'smol-toml';
import { isMain, panic } from '#utils/cli';
import { repoRoot } from '#utils/fs';

if (isMain(import.meta.url)) {
	const args = argv.slice(2);
	if (args.length === 1 && args[0] === '--simulator') {
		await setRustAnayzerConfig('wasm32-unknown-unknown', ['simulator'], {
			// biome-ignore lint/style/useNamingConvention: environment variable
			VERTX_TARGET: 'simulator',
		});
	} else {
		panic(`usage: ${import.meta.file} --simulator`);
	}
}

export async function setRustAnayzerConfig(
	triple: string,
	features: Array<string>,
	env: Record<string, string>,
) {
	const enable = import.meta.env.VERTX_SET_RA_FEATURES ?? 'true';
	if (enable === 'false' || enable === '0') {
		return;
	}

	const config = {
		env,
		triple,
		features,
	};

	await Promise.all([
		helix(config),
		vscode(config),
		// PRs for more are welcome!
	]);
}

type Config = {
	env: Record<string, string>;
	triple: string;
	features: Array<string>;
};

async function helix({ env, triple, features }: Config) {
	const file = await open('.helix/languages.toml');
	const config = TOML.parse(await file.text());
	const baseKey = ['language-server', 'rust-analyzer', 'config'];

	const allEnv = get(config, [...baseKey, 'cargo', 'extraEnv'], {});
	updateEnv(allEnv, env);

	set(config, [...baseKey, 'cargo', 'target'], triple);

	const allFeatures = get(config, [...baseKey, 'cargo', 'features'], []);
	updateFeatures(allFeatures, features);

	await file.write(TOML.stringify(config));
}

async function vscode({ env, triple, features }: Config) {
	const file = await open('.vscode/settings.json');
	const raw = await file.text();
	const config = JSON.parse(raw === '' ? '{}' : raw);

	const allEnv = get(config, ['rust-analyzer.cargo.extraEnv'], {});
	updateEnv(allEnv, env);

	set(config, ['rust-analyzer.cargo.target'], triple);

	const allFeatures = get(config, ['rust-analyzer.cargo.features'], []);
	updateFeatures(allFeatures, features);

	await file.write(JSON.stringify(config));
}

async function open(path: string) {
	const absolute = `${repoRoot}/${path}`;
	if (!(await exists(absolute))) {
		await mkdir(dirname(absolute), { recursive: true });
		await Bun.write(absolute, '');
	}
	return Bun.file(absolute);
}

/** Get a nested value from an object by path, creating missing objects along the way and assigning a default value if needed. */
function get(
	// biome-ignore lint/suspicious/noExplicitAny: impossible to know what type this should be
	from: any,
	path: Array<string>,
	defaultValue: unknown,
) {
	return nested(from, path, (obj, key) => {
		obj[key] ??= defaultValue;
		return obj[key];
	});
}

/** Set a value by path, creating missing objects */
function set(
	// biome-ignore lint/suspicious/noExplicitAny: impossible to know what type this should be
	from: any,
	path: Array<string>,
	value: unknown,
) {
	return nested(from, path, (obj, key) => {
		obj[key] = value;
	});
}

function nested<T>(
	// biome-ignore lint/suspicious/noExplicitAny: impossible to know what type this should be
	from: any,
	path: Array<string>,
	callback: <K extends string>(obj: Record<K, unknown>, key: K) => T,
	read: Array<string> = [],
): T {
	const [key, ...rest] = path;

	if (key == null) {
		throw new TypeError('path cannot be empty');
	}

	if (from == null || typeof from !== 'object') {
		const errorKey = read.map((s) => `'${s}'`).join('.');
		throw new TypeError(`${errorKey} is not an object`);
	}

	if (rest.length === 0) {
		return callback(from, key);
	}

	from[key] ??= {};
	return nested(from[key], rest, callback, [...read, key]);
}

function updateEnv(current: unknown, add: Record<string, string>) {
	if (!isStrMap(current)) {
		throw new Error(
			'rust-analyzer config cargo.extraEnv must be a map of strings',
		);
	}

	Object.assign(current, add);
}

function updateFeatures(current: unknown, add: Array<string>) {
	if (!isStrArray(current)) {
		throw new Error(
			'rust-analyzer.config.cargo.features must be an array of strings',
		);
	}

	// Remove existing vertx/* entries
	let i = 0;
	while (i < current.length) {
		// biome-ignore lint/style/noNonNullAssertion: length is checked
		if (current[i]!.startsWith('vertx/')) {
			current.splice(i, 1);
			continue;
		}

		// biome-ignore lint/style/noNonNullAssertion: splice always continues, so the length check is still valid
		if (!current[i]!.includes('/')) {
			console.warn(`Ignoring unqualified feature: '${current[i]}'`);
		}

		i += 1;
	}

	current.push(...add.map((f) => `vertx/${f}`));
}

function isStrArray(value: unknown): value is Array<string> {
	if (!Array.isArray(value)) {
		return false;
	}

	for (const x of value) {
		if (typeof x !== 'string') {
			return false;
		}
	}

	return true;
}

function isStrMap(value: unknown): value is Record<string, string> {
	if (value == null || typeof value !== 'object' || Array.isArray(value)) {
		return false;
	}

	for (const [k, v] of Object.entries(value)) {
		if (typeof k !== 'string' || typeof v !== 'string') {
			return false;
		}
	}

	return true;
}
