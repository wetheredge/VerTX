#!/usr/bin/env bun

// Hack to remove any assets that aren't actually used. Ideally, they wouldn't get built.

import { rm } from 'node:fs/promises';
import { assetPaths, outDir, pathToRoute, prettySize } from './utils';

type Asset = {
	path: string;
	route: string;
	type: string;
	bytes: number;
	contents: string;
	parents: ParentSet;
};
type ParentSet = Array<ParentSet | typeof used>;
const used = Symbol('used');

const assets = new Set<Asset>();

for await (const rawPath of assetPaths()) {
	const path = `${outDir}/${rawPath}`;
	const file = Bun.file(path);
	const asset: Asset = {
		path,
		route: pathToRoute(rawPath),
		type: file.type,
		bytes: file.size,
		contents: await file.text(),
		parents: [],
	};

	assets.add(asset);
}

for (const needle of assets) {
	if (needle.type.startsWith('text/html')) {
		needle.parents.push(used);
	} else {
		for (const haystack of assets.values()) {
			if (haystack === needle) {
				// Don't count an asset as it's own parent
				continue;
			}

			if (haystack.contents.includes(needle.route)) {
				needle.parents.push(haystack.parents);
			}
		}
	}
}

const pruned = [];
for (const asset of assets) {
	if (asset.parents.flat(assets.size).length === 0) {
		await rm(asset.path);
		pruned.push(asset);
	}
}

if (pruned.length > 0) {
	const size = pruned.reduce((sum, asset) => sum + asset.bytes, 0);
	console.info(
		`Pruned ${pruned.length} assets totaling ${prettySize(size)}:`,
	);
	for (const asset of pruned) {
		console.info(`  ${asset.route}`);
	}
}
