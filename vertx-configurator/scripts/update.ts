#!/usr/bin/env bun

import { chdir, exit, stdout } from 'node:process';
import { $ } from 'bun';

const branch = 'updates';

const anyChanges =
	await $`git diff --exit-code --quiet && git diff --cached --exit-code --quiet`
		.nothrow()
		.quiet();
if (anyChanges.exitCode !== 0) {
	panic('There are uncommitted changes');
}

const currentBranch = await $`git rev-parse --abbrev-ref HEAD`.text();
if (currentBranch.trim() !== 'main') {
	panic('Not on main branch');
}

const branchExists = await $`git rev-parse --verify --quiet ${branch}`
	.nothrow()
	.quiet();
if (branchExists.exitCode === 0) {
	panic(`Branch ${branch} already exists`);
}

const repoRoot = (await $`git rev-parse --show-toplevel`.text()).trim();
chdir(repoRoot);
await $`git switch -c updates`.quiet();
const cargoState = await getCargoState();
await asdf();
await cargoBin(cargoState);
await dprint();
await npm();
await cargo(cargoState);
await cargoState.finish();

async function asdf() {
	const { dep, updateWorkflows } = group('asdf');

	const path = '.tool-versions';
	const commentPattern = /\s*#.*$/;
	const isVersion = /^\d/;

	const file = await Bun.file(path).text();
	const lines = file.split('\n');
	for (let i = 0; i < lines.length; i++) {
		const [line, comment] = splitComment(lines[i], commentPattern);
		if (!line) {
			continue;
		}

		const [tool, version] = line.split(' ');
		const done = dep(tool, version);

		const latest = (await $`asdf list-all ${tool}`.text())
			.split('\n')
			.filter((l) => isVersion.test(l))
			.at(-1)!;

		if (version !== latest) {
			lines[i] = `${tool} ${latest}${comment}`;
			await Promise.all([
				Bun.write(path, lines.join('\n')),
				updateWorkflows(tool, version, latest),
			]);
		}

		await done(latest);
	}
}

async function cargoBin(state: CargoState) {
	await cargoUpdateImpl({
		state,
		group: 'cargo-bin',
		path: 'Cargo.toml',
		section: 'workspace.metadata.bin',
	});
}

async function dprint() {
	const { dep } = group('dprint');

	const path = '.dprint.json';
	const pattern =
		/https:\/\/plugins\.dprint\.dev\/(?<name>\w+)-(?<version>(?:\d+\.){2}\d+)\.wasm/;

	const lines = (await Bun.file(path).text()).split('\n');
	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const matches = line.match(pattern);
		if (!matches) {
			continue;
		}

		const { name, version } = matches.groups as Record<
			'name' | 'version',
			string
		>;
		const done = dep(name, version);

		const response = (await fetch(
			`https://plugins.dprint.dev/dprint/dprint-plugin-${name}/latest.json`,
		).then((r) => r.json())) as Record<'url' | 'version', string>;
		const latest = response.version;

		if (version !== latest) {
			lines[i] = line.replace(pattern, response.url);
			await Bun.write(path, lines.join('\n'));
		}

		await done(latest);
	}
}

async function npm() {
	chdir('vertx-configurator');

	const kinds = [
		['npm', 'dependencies', ''],
		['npm(dev)', 'devDependencies', '--dev'],
	] as const;
	const packageJson = async () =>
		JSON.parse(await Bun.file('package.json').text()) as Record<
			(typeof kinds)[number][1],
			Record<string, string>
		>;

	const data = await packageJson();
	for (const [groupName, kind, kindFlag] of kinds) {
		const { dep } = group(groupName);
		for (const [name, current] of Object.entries(data[kind])) {
			const rawCurrent = rawVersion(current);
			const done = dep(name, rawCurrent);

			await $`bun add --exact ${kindFlag} ${name} && bun run -b biome format --write package.json && git restore bun.lockb`.quiet();

			const newData = await packageJson();
			const latest = newData[kind][name] as string;

			if (rawCurrent === latest) {
				await $`git restore package.json`.quiet();
			}

			await done(latest);
		}
	}

	await $`rm bun.lockb && bun install`.quiet();
	await commit('Recreate bun.lockb');
	console.info('Recreated bun.lockb');

	chdir(repoRoot);
}

async function cargo(state: CargoState) {
	await cargoUpdateImpl({
		state,
		group: 'cargo(workspace)',
		path: 'Cargo.toml',
		section: 'workspace.dependencies',
	});

	const metadata =
		await $`cargo metadata --no-deps --format-version 1`.text();
	const members = JSON.parse(metadata).packages.map(
		(p: { name: string }) => p.name,
	);

	for (const dir of members) {
		await cargoUpdateImpl({
			state,
			group: `cargo(${dir})`,
			path: `${dir}/Cargo.toml`,
			section: 'dependencies',
		});
		await cargoUpdateImpl({
			state,
			group: `cargo(${dir} build)`,
			path: `${dir}/Cargo.toml`,
			section: 'build-dependencies',
		});
		await cargoUpdateImpl({
			state,
			group: `cargo(${dir} dev)`,
			path: `${dir}/Cargo.toml`,
			section: 'dev-dependencies',
		});
	}

	await $`cargo generate-lockfile`.quiet();
	await commit('Recreate Cargo.lock');
	console.info('Recreated Cargo.lock');
}

type CargoState = {
	getLatest: (dep: string) => Promise<string>;
	finish: () => Promise<void>;
};

async function getCargoState(): Promise<CargoState> {
	const dir = (await $`mktemp --directory`.text()).trim();
	await $`cargo init --vcs none --name updates ${dir}`.quiet();
	const cargoTomlLastLine = () =>
		Bun.file(`${dir}/Cargo.toml`)
			.text()
			.then((s) => s.trim().split('\n').at(-1)!);

	if ((await cargoTomlLastLine()) !== '[dependencies]') {
		panic("Generated Cargo.toml doesn't end with [dependencies] table");
	}

	return {
		async getLatest(dep) {
			await $`cargo add ${dep}`.cwd(dir).quiet();
			const latest = (await cargoTomlLastLine()).split('"').at(1)!;
			await $`cargo rm ${dep}`.cwd(dir).quiet();
			return latest;
		},
		async finish() {
			await $`rm -r ${dir}`.quiet();
		},
	};
}

async function cargoUpdateImpl(args: {
	state: CargoState;
	group: string;
	path: string;
	section: string;
	updateWorkflows?: boolean;
}) {
	const { dep, updateWorkflows } = group(args.group);

	const commentPattern = /\s*#.*$/;
	const depPattern =
		/^(\s*)([-_a-z0-9]+)(\s*=\s*(?:\{.*version\s*=\s*)?")([^"]+)(.*)/;
	const crateId = 1;
	const versionId = 3;

	const lines = (await Bun.file(args.path).text()).split('\n');
	const start = lines
		.map((s) => s.replace(/\s/, ''))
		.indexOf(`[${args.section}]`);
	for (let i = start + 1; i >= 0 && i < lines.length; i++) {
		const [line, comment] = splitComment(lines[i], commentPattern);

		if (line.trim().startsWith('[')) {
			break;
		}

		const matches = line.match(depPattern)?.slice(1);
		if (!matches) {
			continue;
		}
		const crate = matches[crateId];
		const current = matches[versionId];
		const rawCurrent = rawVersion(current);

		const done = dep(crate, rawCurrent);
		const latest = await args.state.getLatest(crate);

		if (rawCurrent !== latest) {
			const exactLatest = latest.replace(/^[~^=]?/, '=');
			lines[i] = matches.with(versionId, exactLatest).join('') + comment;
			await Bun.write(args.path, lines.join('\n'));

			if (args.updateWorkflows) {
				await updateWorkflows(crate, rawCurrent, latest);
			}
		}

		await done(latest);
	}
}

function group(group: string) {
	console.info(`Updating ${group} dependencies`);
	let updatedAny = false;

	return {
		dep(dep: string, current: string) {
			stdout.write(`\t${dep} ${current}`);
			return async (latest: string) => {
				if (current === latest) {
					console.info(' is current');
				} else {
					console.info(` -> ${latest}`);

					if (import.meta.env.CI) {
						if (!updatedAny) {
							const md = `### \`${group}\`\ndependency|from|to\n-|-|-`;
							await $`echo ${md} >> ${repoRoot}/.updates.md`.quiet();
							updatedAny = true;
						}
						const md = `\`${dep}\`|${current}|${latest}`;
						await $`echo ${md} >> ${repoRoot}/.updates.md`.quiet();
					}
				}

				await commit(`${group}: ${dep} ${current} -> ${latest}`);
			};
		},
		async updateWorkflows(dep: string, current: string, latest: string) {
			const matches =
				$`rg --line-number '# dep:${group}:${dep}$' .github/workflows`.nothrow();
			for await (const match of matches.lines()) {
				if (!match) {
					continue;
				}

				const [file, line] = match.split(':');
				const escapedVersion = current.replaceAll('.', '\\.');
				Bun.spawnSync([
					'sed',
					'-Ei',
					`${line}s/(.*)${escapedVersion}/\\1${latest}/`,
					file,
				]);
			}
		},
	};
}

async function commit(message: string) {
	await $`git add -u`.quiet();
	const stagedChanges = await $`git diff --cached --exit-code --quiet`
		.nothrow()
		.quiet();
	if (stagedChanges.exitCode !== 0) {
		await $`git commit -m ${message}`.quiet();
	}
}

function rawVersion(v: string) {
	return v.replace(/^[=~^]/, '');
}

function splitComment(line: string, pattern: RegExp): [string, string] {
	const commentStart = line.search(pattern);
	return commentStart >= 0
		? [line.slice(0, commentStart), line.slice(commentStart)]
		: [line, ''];
}

function panic(message: string) {
	console.error(message);
	if (import.meta.env.CI) {
		stdout.write(`::error::${message}\n`);
	}
	exit(1);
}
