import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { $ } from 'bun';
import type { Listr, ListrTask } from 'listr2';
import { getXtensaToolchainVersion, humanBytes } from '../utils';
import type { Context } from './main';

const version = await getXtensaToolchainVersion();

export default (context: Context): ListrTask => {
	const namespace = 'rust';
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

			const rustcVersion = await $`rustc +stable -vV`.text();
			const target = rustcVersion.match(/host:\s*(.*)/)![1];

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
							{
								title: 'download',
								skip: () => Bun.file(downloadPath).exists(),
								async task(_ctx, task) {
									const response = await fetch(asset.url);
									const writer =
										Bun.file(downloadPath).writer();

									if (!response.ok) {
										throw new Error(
											`fetch failed: ${response.status}`,
										);
									}

									const rawLength = Number.parseInt(
										response.headers.get(
											'Content-Length',
										) ?? '',
										10,
									);
									const length = rawLength
										? humanBytes(rawLength)
										: null;

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
							},
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
					},
				},
			]);
		},
	};
};

function getTempDir(prefix: string, dir = os.tmpdir()): Promise<string> {
	return fs.mkdtemp(path.join(dir, prefix));
}
