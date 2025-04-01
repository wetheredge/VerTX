#!/usr/bin/env bun

import { join } from 'node:path';
import { exit } from 'node:process';
import { $ } from 'bun';
import {
	baseOutDir,
	fileAppend,
	fsReplaceSymlink,
	isMain,
	orExit,
	repoRoot,
} from './utils.ts';

const tool = (bin: string) =>
	import.meta.env.CI === 'true' ? bin : join(repoRoot, '.tools', 'bin', bin);
const firmwareOutDir = join(baseOutDir, 'firmware');

export async function build(command: string, args: Array<string> = []) {
	const cargo = $`cargo ${command} -p vertx -Zbuild-std=std,panic_abort --target wasm32-unknown-unknown -F simulator ${args}`;
	await orExit(
		cargo.env({
			// biome-ignore lint/style/useNamingConvention:
			CARGO_TERM_COLOR: 'always',
			...process.env,
		}),
	);

	if (command === 'build') {
		const isRelease = args.includes('-r') || args.includes('--release');
		const profile = isRelease ? 'release' : 'debug';

		const outName = `simulator_${profile}`;
		const outDir = join(firmwareOutDir, outName);
		const bindgen = $`${tool('wasm-bindgen')} --out-dir ${outDir} --target web target/wasm32-unknown-unknown/${profile}/vertx.wasm`;
		await orExit(bindgen.cwd(repoRoot));

		await Promise.all([
			fileAppend(
				join(outDir, 'vertx.d.ts'),
				'\nexport const memoryName: "memory";',
			),
			fileAppend(
				join(outDir, 'vertx.js'),
				'\nexport const memoryName = "memory";',
			),
		]);

		if (isRelease) {
			const passes = [
				'--converge',
				'--const-hoisting',
				'--dae-optimizing',
				'--optimize-added-constants-propagate',
				'--optimize-instructions',
				'--reorder-functions',
				'--reorder-locals',
				'--strip-target-features',
				'--strip-producers',
				'--minify-imports-and-exports-and-modules',
			];
			const wasm = 'vertx_bg.wasm';
			const wasmOpt =
				$`${tool('wasm-opt')} -O3 ${passes} --output ${wasm} ${wasm}`
					.cwd(outDir)
					.nothrow();

			const renames = new Map();
			for await (const line of wasmOpt.lines()) {
				const [from, to] = line.split(' => ');
				if (to && from !== to) {
					renames.set(from, to);
				}
			}
			const wasmOptResult = await wasmOpt;
			if (wasmOptResult.exitCode !== 0) {
				process.stderr.write(wasmOptResult.stderr);
				exit(wasmOptResult.exitCode);
			}

			const wasmDts = Bun.file(join(outDir, 'vertx_bg.wasm.d.ts'));
			let wasmDtsContents = await wasmDts.text();
			for (const [from, to] of renames) {
				wasmDtsContents = wasmDtsContents.replaceAll(from, to);
			}
			await wasmDts.write(wasmDtsContents);

			const dts = Bun.file(join(outDir, 'vertx.d.ts'));
			let dtsContents = await dts.text();
			for (const [from, to] of renames) {
				dtsContents = dtsContents
					.replaceAll(`readonly ${from}`, `readonly ${to}`)
					.replaceAll(`'${from}'`, `'${to}'`)
					.replaceAll(`"${from}"`, `"${to}"`);
			}
			await Bun.write(dts, dtsContents);

			const js = Bun.file(join(outDir, 'vertx.js'));
			let jsContents = await js.text();
			jsContents = jsContents.replaceAll('imports.wbg', 'imports.a');
			for (const [from, to] of renames) {
				jsContents = jsContents
					.replaceAll(`.${from}`, `.${to}`)
					.replaceAll(`'${from}'`, `'${to}'`)
					.replaceAll(`"${from}"`, `"${to}"`);
			}
			await Bun.write(js, jsContents);
		}

		await fsReplaceSymlink(outName, join(firmwareOutDir, 'simulator'));
	}
}

if (isMain(import.meta.url)) {
	const usage = `usage: scripts/${import.meta.file} [--command build/clippy/…] [...args]`;
	let args = Bun.argv.slice(2);

	if (args[0] === '--help' || args[0] === '-h') {
		console.info(usage);
		exit(0);
	}

	let command = 'build';
	if (args[0] === '--command') {
		if (args[1] == null) {
			console.error(usage);
			exit(1);
		}

		command = args[1];
		args = args.slice(2);
	} else if (args[0]?.startsWith('--command=')) {
		// biome-ignore lint/style/noNonNullAssertion: known to include an =
		command = args[0].split('=', 2)[1]!;
		args = args.slice(1);
	}

	await build(command, args);
}
