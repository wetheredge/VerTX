#!/usr/bin/env bun

import { join } from 'node:path';
import { exit } from 'node:process';
import { $ } from 'bun';
import { fileAppend, isMain, orExit, repoRoot } from './utils.ts';

const toolsPath = import.meta.env.CI === 'true' ? '' : '.tools/bin/';

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

		const outputDir = join(repoRoot, 'target/simulator');
		const bindgen = $`${toolsPath}wasm-bindgen --out-dir ${outputDir} --target web target/wasm32-unknown-unknown/${profile}/vertx.wasm`;
		await orExit(bindgen.cwd(repoRoot));

		await Promise.all([
			fileAppend(
				join(outputDir, 'vertx.d.ts'),
				'\nexport const memoryName: "memory";',
			),
			fileAppend(
				join(outputDir, 'vertx.js'),
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
			const wasm = 'target/simulator/vertx_bg.wasm';
			const wasmOpt =
				$`${toolsPath}wasm-opt -O3 ${passes} --output ${wasm} ${wasm}`
					.cwd(repoRoot)
					.nothrow();

			const renames = new Map();
			for await (const line of wasmOpt.lines()) {
				const [from, to] = line.split(' => ');
				if (to && from !== to) {
					renames.set(from, to);
				}
			}
			await orExit(wasmOpt);

			const wasmDts = Bun.file(join(outputDir, 'vertx_bg.wasm.d.ts'));
			let wasmDtsContents = await wasmDts.text();
			for (const [from, to] of renames) {
				wasmDtsContents = wasmDtsContents.replaceAll(from, to);
			}
			await wasmDts.write(wasmDtsContents);

			const dts = Bun.file(join(outputDir, 'vertx.d.ts'));
			let dtsContents = await dts.text();
			for (const [from, to] of renames) {
				dtsContents = dtsContents
					.replaceAll(`readonly ${from}`, `readonly ${to}`)
					.replaceAll(`'${from}'`, `'${to}'`)
					.replaceAll(`"${from}"`, `"${to}"`);
			}
			await Bun.write(dts, dtsContents);

			const js = Bun.file(join(outputDir, 'vertx.js'));
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
