#!/usr/bin/env bun

import { existsSync, rmSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { exit } from 'node:process';
import { $, fileURLToPath } from 'bun';
import * as chip2Target from '../.config/chips.json';
import { type Pin, type Target, isPin, schema } from './target-schema.ts';
import { isMain, repoRoot } from './utils.ts';

async function codegen(target: Target) {
	const file = Bun.file(`${repoRoot}/vertx/src/hal/codegen.rs`);
	if (await file.exists()) {
		await file.delete();
	}
	const out = file.writer();

	const gpioPrefix = target.chip.startsWith('esp')
		? 'GPIO'
		: target.chip.startsWith('rp')
			? 'PIN_'
			: 'P';
	const isStm32 = target.chip.startsWith('stm32');

	type Tree = { [key: string | symbol]: string | Pin | Tree };
	function walk(tree: Tree, parent = new Array<string>()) {
		for (const [self, value] of Object.entries(tree)) {
			const key = [...parent, self];
			const keyStr = key.join('.');
			if (typeof value === 'string') {
				out.write(`    ($p:expr, ${keyStr}) => { $p.${value} };\n`);
			} else if (isPin(value)) {
				const pin = `${gpioPrefix}${value.pin}`;
				out.write(`    ($p:expr, ${keyStr}) => { $p.${pin} };\n`);
				if (isStm32) {
					const exti = pin.toString().replace(/^[a-zA-Z]+/, '');
					out.write(
						`    ($p:expr, ExtiInput(${keyStr} $(, $arg:expr)* $(,)?)) => `,
					);
					out.write(
						`{ ExtiInput::new($p.${pin}, $p.EXTI${exti}, $($arg)*) };\n`,
					);
				}
			} else {
				walk(value, key);
			}
		}
	}

	out.write('macro_rules! target {\n');
	walk(target);
	out.write('}\n');
	await out.flush();
}

export async function build(
	command: string,
	targetName: string,
	rawTarget: unknown,
	args: Array<string> = [],
) {
	const target = schema.parse(rawTarget);
	await codegen(target);

	const chip = getChipInfo(target.chip);
	const features = getFeatures(target).join(' ');

	const cfgs = [
		target.sd.spi,
		'i2c' in target.display && target.display.i2c,
		'spi' in target.display && target.display.spi,
	].flatMap((p) => (p ? [`--cfg=peripheral="${p}"`] : []));

	const rustflags = [
		process.env.RUSTFLAGS,
		chip.cpu && `-Ctarget-cpu=${chip.cpu}`,
		...cfgs,
	]
		.filter((s) => s && s.length > 0)
		.join(' ');

	const cargo =
		await $`cargo ${command} -p vertx -Zbuild-std=alloc,core --target ${chip.target} -F '${features}' ${args}`
			.nothrow()
			.env({
				CARGO_TERM_COLOR: 'always',
				...process.env,
				RUSTFLAGS: rustflags,
				VERTX_TARGET: targetName,
			});
	if (cargo.exitCode !== 0) {
		exit(cargo.exitCode);
	}

	if (command === 'build') {
		const targetDir = fileURLToPath(new URL('../target', import.meta.url));
		const isRelease = args.includes('-r') || args.includes('--release');
		const profile = isRelease ? 'release' : 'debug';

		const from = join(chip.target, profile, 'vertx');
		const to = join(targetDir, 'vertx');

		if (existsSync(to)) {
			rmSync(to);
		}

		symlinkSync(from, to);
	}
}

export function getFeatures(target: Target): Array<string> {
	return [
		`chip-${target.chip}`,
		target.sd.type === 'spi' && 'storage-sd',
		`display-${target.display.driver}`,
	].filter((x) => typeof x === 'string');
}

type ChipInfo = { target: string; cpu?: string };
function getChipInfo(chip: string): ChipInfo {
	const info = (chip2Target as Record<string, string | ChipInfo>)[chip];
	if (info == null) {
		throw new Error(`Missing info for chip '${chip}'`);
	}
	return typeof info === 'string' ? { target: info } : info;
}

if (isMain(import.meta.url)) {
	const usage = `usage: scripts/${import.meta.file} [--command build/clippy/…] <target> [...args]`;
	let args = Bun.argv.slice(2);

	if (args[0] === '--help' || args[0] === '-h') {
		console.info(usage);
		exit(0);
	}

	let command = 'build';
	if (args[0] === '--command') {
		command = args[1];
		args = args.slice(2);
	} else if (args[0]?.startsWith('--command=')) {
		command = args[0].split('=', 2)[1];
		args = args.slice(1);
	}

	if (args.length < 1) {
		console.error(usage);
		exit(1);
	}

	const [targetName, ...buildArgs] = args;
	const targetPath = fileURLToPath(
		new URL(`../targets/${targetName}.toml`, import.meta.url),
	);

	if (!existsSync(targetPath)) {
		console.error(`Cannot find target '${targetName}'`);
		exit(1);
	}

	const target = await import(targetPath);
	await build(command, targetName, target.default, buildArgs);
}
