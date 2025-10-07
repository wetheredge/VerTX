import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { $, fileURLToPath } from 'bun';
import { Listr, type ListrTask } from 'listr2';
import * as versions from '../.config/versions.json';
import { humanBytes, repoRoot } from './utils.ts';

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
		async skip() {
			const output = await $`${dirs.out}/bin/rustc --version`
				.nothrow()
				.text();
			return output.trimEnd().endsWith(`(${version})`);
		},
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
									await $`tar -xf ${downloadPath} -C ${dir} --strip-components 1`.quiet();
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
						for (const dir of extracted) {
							await $`${dir}/install.sh --prefix=${tempDir}`.quiet();
							await fs.rm(dir, { recursive: true });
						}
						fs.rename(tempDir, dirs.out);

						await $`rustup toolchain link vertx ${dirs.out}`.quiet();
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
		async skip() {
			const output =
				await $`${dirs.out}/bin/xtensa-esp32s3-elf-gcc --version`
					.nothrow()
					.text();
			return output.includes(version);
		},
		async task(_ctx, task): Promise<Listr> {
			await fs.mkdir(dirs.download, { recursive: true });

			const llvmTarget = await getLlvmTarget();
			const target = LLVM_TARGET_TO_GCC[llvmTarget];
			if (target == null) {
				const file = path.relative(
					process.cwd(),
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
						await $`tar -xf ${downloadPath} -C ${tempDir} --strip-components 1`.quiet();

						await fs.rm(dirs.out, { recursive: true, force: true });
						await fs.rename(tempDir, dirs.out);
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
			for await (const chunk of response.body ?? []) {
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
	const output = await $`rustc +stable -vV`.text();
	const version = output.match(/host:\s*(.*)/)?.[1];
	if (version == null) {
		throw new Error('failed to get llvm target');
	}
	return version;
}

function getTempDir(prefix: string, dir = os.tmpdir()): Promise<string> {
	return fs.mkdtemp(path.join(dir, prefix));
}
