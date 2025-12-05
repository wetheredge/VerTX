import { existsSync } from 'node:fs';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { cwd } from 'node:process';
import { Writable } from 'node:stream';
import { fileURLToPath } from 'node:url';
import { Listr, type ListrTask } from 'listr2';
import { SubprocessError } from 'nano-spawn';
import { exec } from '#utils/cli';
import { repoRoot } from '#utils/fs';
import versions from '../.config/versions.json' with { type: 'json' };

// See <https://github.com/espressif/crosstool-NG/releases/latest>
const LLVM_TARGET_TO_GCC: Record<string, string> = {
	'x86_64-unknown-linux-gnu': 'x86_64-linux-gnu',
};

const toolsDir = path.join(repoRoot, '.tools');
const getDirs = (namespace: string, version: string) => ({
	out: path.join(toolsDir, namespace),
	download: path.join(
		path.join(repoRoot, '.cache/downloads'),
		namespace,
		version,
	),
});

await fs.mkdir(toolsDir, { recursive: true });

await new Listr([rust(), gcc()]).run();

function rust(): ListrTask {
	const version = versions.rust;
	const dirs = getDirs('rust', version);

	return {
		title: `Xtensa Rust v${version}`,
		skip: () =>
			testCommand([`${dirs.out}/bin/rustc`, '--version'], (stdout) =>
				stdout.trimEnd().endsWith(`(${version})`),
			),
		async task(_ctx, task): Promise<Listr> {
			await fs.mkdir(dirs.download, { recursive: true });

			const target = await getLlvmTarget();
			const assets = [
				{
					name: 'rust-src.tar.xz',
					url: `https://github.com/esp-rs/rust-build/releases/download/v${version}/rust-src-${version}.tar.xz`,
				},
				{
					name: 'rust.tar.xz',
					url: `https://github.com/esp-rs/rust-build/releases/download/v${version}/rust-${version}-${target}.tar.xz`,
				},
			];

			const extracted: Array<string> = [];
			const assetTasks = assets.map((asset): ListrTask => {
				const downloadPath = path.join(dirs.download, asset.name);
				return {
					title: asset.name,
					task: (_ctx, task) => {
						return task.newListr([
							downloadTask(asset.url, downloadPath),
							{
								title: 'extract',
								async task() {
									const tempPrefix = `${asset.name}-`;
									const dir = await getTempDir(tempPrefix);
									await untar(downloadPath, dir, 1);
									extracted.push(dir);
								},
							},
						]);
					},
				};
			});

			return task.newListr([
				...assetTasks,
				{
					title: 'install',
					async task() {
						await fs.rm(dirs.out, { recursive: true, force: true });
						const tempPrefix = '.rust-xtensa-install-';
						const tempDir = await getTempDir(tempPrefix, toolsDir);
						// biome-ignore-start lint/performance/noAwaitInLoops: avoid concurrent modifications
						for (const dir of extracted) {
							await exec(
								`${dir}/install.sh`,
								`--prefix=${tempDir}`,
								{ quiet: true },
							);
							await fs.rm(dir, { recursive: true });
						}
						// biome-ignore-end lint/performance/noAwaitInLoops: _
						fs.rename(tempDir, dirs.out);

						await exec(
							'rustup',
							'toolchain',
							'link',
							'vertx',
							dirs.out,
							{ quiet: true },
						);
					},
				},
			]);
		},
	};
}

function gcc(): ListrTask {
	const namespace = 'gcc';
	const version = versions.gcc;

	const dirs = getDirs(namespace, version);

	return {
		title: `Xtensa GCC v${version}`,
		skip: () =>
			testCommand(
				[`${dirs.out}/bin/xtensa-esp32s3-elf-gcc`, '--version'],
				(stdout) => stdout.includes(version),
			),
		async task(_ctx, task): Promise<Listr> {
			await fs.mkdir(dirs.download, { recursive: true });

			const llvmTarget = await getLlvmTarget();
			const target = LLVM_TARGET_TO_GCC[llvmTarget];
			if (target == null) {
				const file = path.relative(
					cwd(),
					fileURLToPath(import.meta.url),
				);
				throw new Error(
					`Missing mapping from llvm target '${llvmTarget}' to the xtensa gcc name for this platform. Please update ${file} and send a PR`,
				);
			}

			const url = `https://github.com/espressif/crosstool-NG/releases/download/esp-${version}/xtensa-esp-elf-${version}-${target}.tar.xz`;
			const downloadPath = path.join(dirs.download, 'gcc.tar.xz');

			return task.newListr([
				downloadTask(url, downloadPath),
				{
					title: 'install',
					async task() {
						const tempPrefix = `.${namespace}-`;
						const tempDir = await getTempDir(tempPrefix, toolsDir);
						await untar(downloadPath, tempDir, 1);

						await fs.rm(dirs.out, { recursive: true, force: true });
						await fs.rename(tempDir, dirs.out);
					},
				},
			]);
		},
	};
}

async function testCommand(
	cmd: [string, ...Array<string>],
	checkStdout: (s: string) => boolean,
): Promise<boolean> {
	try {
		const { stdout } = await exec(...cmd);
		return checkStdout(stdout);
	} catch (err) {
		if (err instanceof SubprocessError) {
			return false;
		}

		throw err;
	}
}

function downloadTask(url: string, downloadPath: string): ListrTask {
	return {
		title: 'download',
		skip: () => existsSync(downloadPath),
		async task(_ctx, task) {
			const response = await fetch(url);
			const file = await fs.open(downloadPath);

			if (!response.ok) {
				throw new Error(`fetch failed: ${response.status}`);
			}

			const contentLength = response.headers.get('Content-Length') ?? '';
			const rawLength = Number.parseInt(contentLength, 10);
			const length = rawLength ? humanBytes(rawLength) : null;

			if (response.body) {
				const writeStream = file.createWriteStream();
				const writer = Writable.toWeb(writeStream).getWriter();

				let downloaded = 0;
				const progressStream = new WritableStream<ArrayBufferLike>({
					abort: (reason) => writer.abort(reason),
					close: () => writer.close(),
					async write(chunk) {
						await writer.write(chunk);
						downloaded += chunk.byteLength;
						task.output = rawLength
							? `${humanBytes(downloaded)} / ${length}`
							: humanBytes(downloaded);
					},
				});

				response.body.pipeTo(progressStream);
			}

			await file.close();
		},
	};
}

async function untar(file: string, dir: string, strip: number) {
	await exec(
		'tar',
		'--extract',
		`--file=${file}`,
		`--directory=${dir}`,
		`--strip-components=${strip}`,
		{ quiet: true },
	);
}

async function getLlvmTarget(): Promise<string> {
	const { stdout } = await exec('rustc', '+stable', '-vV');
	const version = stdout.match(/host:\s*(.*)/)?.[1];
	if (version == null) {
		throw new Error('failed to get llvm target');
	}
	return version;
}

function getTempDir(prefix: string, dir = os.tmpdir()): Promise<string> {
	return fs.mkdtemp(path.join(dir, prefix));
}

function humanBytes(bytes: number): string {
	const base = 1024;

	if (bytes < base) {
		return `${bytes} B`;
	}

	const prefixes = ['K', 'M', 'G', 'T'];

	let size = bytes;
	let prefix = -1;
	do {
		size /= base;
		prefix++;
	} while (size >= base && prefix + 1 < prefixes.length);

	return `${size.toFixed(2)} ${prefixes[prefix]}iB`;
}
