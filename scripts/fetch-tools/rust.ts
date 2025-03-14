import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { $, fileURLToPath } from 'bun';
import type { Listr, ListrTask } from 'listr2';
import * as versions from '../../.config/xtensa-toolchain.json';
import { humanBytes } from '../utils';
import type { Context } from './main';

// See <https://github.com/espressif/crosstool-NG/releases/latest>
const LLVM_TARGET_TO_GCC: Record<string, string> = {
	'x86_64-unknown-linux-gnu': 'x86_64-linux-gnu',
};

export function rust(context: Context): ListrTask {
	const namespace = 'rust';
	const version = versions.rust;

	const outDir = path.join(context.outDir, namespace);
	const downloadDir = path.join(context.downloadDir, namespace, version);

	return {
		title: `Xtensa Rust v${version}`,
		async skip() {
			const output = await $`${outDir}/bin/rustc --version`
				.nothrow()
				.text();
			return output.trimEnd().endsWith(`(${version})`);
		},
		async task(_ctx, task): Promise<Listr> {
			await fs.mkdir(downloadDir, { recursive: true });

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

			const extracted = new Array<string>();

			const assetTasks = assets.map((asset): ListrTask => {
				const downloadPath = path.join(downloadDir, asset.name);
				return {
					title: asset.name,
					task: (_ctx, task) =>
						task.newListr([
							downloadTask(asset.url, downloadPath),
							{
								title: 'extract',
								async task() {
									const dir = await getTempDir(
										`${asset.name}-`,
									);
									await $`tar -xf ${downloadPath} -C ${dir} --strip-components 1`.quiet();

									extracted.push(dir);
								},
							},
						]),
				};
			});

			return task.newListr([
				...assetTasks,
				{
					title: 'install',
					async task() {
						await fs.rm(outDir, { recursive: true, force: true });
						await fs.mkdir(context.outDir, { recursive: true });
						const installDir = await getTempDir(
							'.rust-xtensa-install-',
							context.outDir,
						);
						for (const dir of extracted) {
							await $`${dir}/install.sh --prefix=${installDir}`.quiet();
							await fs.rm(dir, { recursive: true });
						}
						fs.rename(installDir, outDir);

						await $`rustup toolchain link vertx ${outDir}`.quiet();
					},
				},
			]);
		},
	};
}

export function gcc(context: Context): ListrTask {
	const namespace = 'gcc';
	const version = versions.gcc;

	const outDir = path.join(context.outDir, namespace);
	const downloadDir = path.join(context.downloadDir, namespace, version);

	return {
		title: `Xtensa GCC v${version}`,
		async skip() {
			const output =
				await $`${outDir}/bin/xtensa-esp32s3-elf-gcc --version`
					.nothrow()
					.text();
			return output.includes(version);
		},
		async task(_ctx, task): Promise<Listr> {
			await fs.mkdir(downloadDir, { recursive: true });

			const llvmTarget = await getLlvmTarget();
			if (!(llvmTarget in LLVM_TARGET_TO_GCC)) {
				const file = path.relative(
					process.cwd(),
					fileURLToPath(import.meta.url),
				);
				throw new Error(
					`Missing mapping from llvm target '${llvmTarget}' to the xtensa gcc name. Please update ${file} and send a PR`,
				);
			}
			const target = LLVM_TARGET_TO_GCC[llvmTarget];

			const url = `https://github.com/espressif/crosstool-NG/releases/download/esp-${version}/xtensa-esp-elf-${version}-${target}.tar.xz`;
			const downloadPath = path.join(downloadDir, 'gcc.tar.xz');

			return task.newListr([
				downloadTask(url, downloadPath),
				{
					title: 'install',
					async task() {
						const tempDir = await getTempDir(
							`.${namespace}-`,
							context.outDir,
						);
						await $`tar -xf ${downloadPath} -C ${tempDir} --strip-components 1`.quiet();

						await fs.rm(outDir, { recursive: true, force: true });
						await fs.rename(tempDir, outDir);
					},
				},
			]);
		},
	};
}

function downloadTask(url: string, downloadPath: string): ListrTask {
	return {
		title: 'download',
		skip: () => Bun.file(downloadPath).exists(),
		async task(_ctx, task) {
			const response = await fetch(url);
			const writer = Bun.file(downloadPath).writer();

			if (!response.ok) {
				throw new Error(`fetch failed: ${response.status}`);
			}

			const contentLength = response.headers.get('Content-Length') ?? '';
			const rawLength = Number.parseInt(contentLength, 10);
			const length = rawLength ? humanBytes(rawLength) : null;

			let downloaded = 0;
			for await (const chunk of response.body!) {
				writer.write(chunk);
				downloaded += chunk.byteLength;
				if (rawLength) {
					task.output = `${humanBytes(downloaded)} / ${length}`;
				} else {
					task.output = `${humanBytes(downloaded)}`;
				}
			}

			await writer.end();
		},
	};
}

async function getLlvmTarget(): Promise<string> {
	const rustcVersion = await $`rustc +stable -vV`.text();
	return rustcVersion.match(/host:\s*(.*)/)![1];
}

function getTempDir(prefix: string, dir = os.tmpdir()): Promise<string> {
	return fs.mkdtemp(path.join(dir, prefix));
}
