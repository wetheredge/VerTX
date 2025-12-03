#!/usr/bin/env node

import { appendFile, readFile, writeFile } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { argv, env, exit } from 'node:process';
import { parseArgs } from 'node:util';
import { SubprocessError } from 'nano-spawn';
import { cargoBuild } from '#utils/cargo';
import { exec, isMain, orExit } from '#utils/cli';
import { baseOutDir, fsReplaceSymlink, repoRoot } from '#utils/fs';

const firmwareOutDir = join(baseOutDir, 'firmware');

export async function build(command: string, release?: boolean) {
	await cargoBuild({
		command,
		buildStd: 'std,panic_abort',
		target: 'wasm32-unknown-unknown',
		features: ['simulator'],
		release,
		env: {
			...env,
			// biome-ignore lint/style/useNamingConvention: env var
			VERTX_TARGET: 'simulator',
		},
	});

	if (command === 'build') {
		const profile = release ? 'release' : 'debug';

		const outName = `simulator_${profile}`;
		const outDir = join(firmwareOutDir, outName);
		await orExit(
			exec(
				'wasm-bindgen',
				`--out-dir=${outDir}`,
				'--target=web',
				`target/wasm32-unknown-unknown/${profile}/vertx.wasm`,
				{ cwd: repoRoot },
			),
		);

		await Promise.all([
			appendFile(
				join(outDir, 'vertx.d.ts'),
				'\nexport const memoryName: "memory";',
				{ flush: true },
			),
			appendFile(
				join(outDir, 'vertx.js'),
				'\nexport const memoryName = "memory";',
				{ flush: true },
			),
		]);

		if (release) {
			const passes = [
				'--converge',
				'--low-memory-unused',
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
			const wasmOpt = exec(
				'wasm-opt',
				'-O3',
				...passes,
				`--output=${wasm}`,
				wasm,
				{ cwd: outDir, stdout: 'pipe' },
			);

			const renames = new Map<string, string>();
			try {
				for await (const line of wasmOpt.stdout) {
					const [from, to] = line.split(' => ');
					if (to && from !== to) {
						renames.set(from, to);
					}
				}
				await wasmOpt;
			} catch (err) {
				if (err instanceof SubprocessError && err.exitCode != null) {
					console.error('wasm-opt failed');
					exit(err.exitCode);
				}
				throw err;
			}

			type Updates = Array<[from: string, to: string]>;
			const updateFile = async (
				file: string,
				getUpdates: (from: string, to: string) => Updates,
				once: Updates = [],
			) => {
				const path = join(outDir, file);
				let contents = await readFile(path, { encoding: 'utf8' });
				const apply = (updates: Updates) => {
					for (const [from, to] of updates) {
						contents = contents.replaceAll(from, to);
					}
				};

				apply(once);
				for (const [from, to] of renames) {
					apply(getUpdates(from, to));
				}

				await writeFile(path, contents);
			};

			await Promise.all([
				updateFile('vertx_bg.wasm.d.ts', (from, to) => [[from, to]]),
				updateFile('vertx.d.ts', (from, to) => [
					[`readonly ${from}`, `readonly ${to}`],
					[`'${from}'`, `'${to}'`],
					[`"${from}"`, `"${to}"`],
				]),
				updateFile(
					'vertx.js',
					(from, to) => [
						[`.${from}`, `.${to}`],
						[`'${from}'`, `'${to}'`],
						[`"${from}"`, `"${to}"`],
					],
					[['imports.wbg', 'imports.a']],
				),
			]);
		}

		await fsReplaceSymlink(outName, join(firmwareOutDir, 'simulator'));
	}
}

if (isMain(import.meta.url)) {
	const usage = `usage: scripts/${basename(import.meta.filename)} [--command build/clippy/…] [...args]`;

	const { values } = parseArgs({
		args: argv.slice(2),
		options: {
			help: { short: 'h', type: 'boolean' },
			command: { type: 'string', default: 'build' },
			release: { short: 'r', type: 'boolean' },
		},
	});

	if (values.help) {
		console.info(usage);
		exit(0);
	}

	await build(values.command, values.release);
}
