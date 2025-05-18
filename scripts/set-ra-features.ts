#!/usr/bin/env bun

import { exists, mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import TOML from 'smol-toml';
import { isMain, repoRoot } from './utils.ts';

if (isMain(import.meta.url)) {
	const features = process.argv.slice(2);
	await setRustAnayzerFeatures(features);
}

export async function setRustAnayzerFeatures(features: Array<string>) {
	const env = Bun.env.VERTX_SET_RA_FEATURES ?? 'true';
	if (['0', 'false'].includes(env)) {
		return;
	}

	await Promise.all([
		helix(features),
		vscode(features),
		// PRs for more are welcome!
	]);
}

async function helix(features: Array<string>) {
	const file = await open('.helix/languages.toml');
	const config = TOML.parse(await file.text());
	const allFeatures = get(
		config,
		['language-server', 'rust-analyzer', 'config', 'cargo', 'features'],
		[],
	);
	updateFeatures(allFeatures, features);
	await file.write(TOML.stringify(config));
}

async function vscode(features: Array<string>) {
	const file = await open('.vscode/settings.json');
	const raw = await file.text();
	const config = JSON.parse(raw === '' ? '{}' : raw);
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

function get(
	// biome-ignore lint/suspicious/noExplicitAny:
	from: any,
	path: Array<string>,
	defaultValue: unknown,
	read: Array<string> = [],
) {
	const [key, ...rest] = path;

	if (key == null) {
		return from;
	}

	if (!(key in from)) {
		if (from == null || typeof from !== 'object') {
			const errorKey = read.map((s) => `'${s}'`).join('.');
			throw new TypeError(`${errorKey} is not an object`);
		}

		if (rest.length === 0) {
			from[key] = defaultValue;
			return from[key];
		}

		from[key] = {};
	}

	return get(from[key], rest, defaultValue, [...read, key]);
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
		if (current[i].startsWith('vertx/')) {
			current.splice(i, 1);
			continue;
		}

		if (!current[i].includes('/')) {
			console.warn(`Ignoring unqualified feature: '${current[i]}'`);
		}

		i += 1;
	}

	current.push(...add.map((f) => `vertx/${f}`));
}

// biome-ignore lint/suspicious/noExplicitAny:
function isStrArray(value: any): value is Array<string> {
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
