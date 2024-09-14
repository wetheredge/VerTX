#!/usr/bin/env bun

import { chdir, exit, stdout } from 'node:process';
import { request as ghRequest } from '@octokit/request';
import { $, Glob } from 'bun';
import { getRepoRoot } from './utils';

const missingTools = [
	'asdf',
	'bun',
	'cargo',
	'echo',
	'git',
	'mktemp',
	'rm',
].filter((tool) => Bun.which(tool) == null);
if (missingTools.length > 0) {
	panic(`Missing required tools: ${missingTools.join(', ')}`);
}

const branch = 'updates';

const anyChanges =
	await $`git diff --exit-code --quiet && git diff --cached --exit-code --quiet`
		.nothrow()
		.quiet();
if (anyChanges.exitCode !== 0) {
	panic('There are uncommitted changes');
}

const currentBranch = await $`git rev-parse --abbrev-ref HEAD`.text();
if (!['main', 'ci'].includes(currentBranch.trim())) {
	panic('Not on main or ci branch');
}

const branchExists = await $`git rev-parse --verify --quiet ${branch}`
	.nothrow()
	.quiet();
if (branchExists.exitCode === 0) {
	panic(`Branch ${branch} already exists`);
}

const repoRoot = await getRepoRoot();
chdir(repoRoot);
await $`git switch -c updates`.quiet();
const cargoState = await getCargoState();
await asdf();
await cargoBin(cargoState);
await dprint();
await npm();
await cargo(cargoState);
await githubActions();
await cargoState.finish();

async function asdf() {
	const { dep, updateWorkflows } = group('asdf');

	const path = '.tool-versions';
	const commentPattern = /\s*#.*$/;
	const isVersion = /^\d/;

	const file = Bun.file(path);
	const contents = await file.text();
	const lines = contents.split('\n');
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
				Bun.write(file, lines.join('\n')),
				updateWorkflows(tool, latest),
			]);
		}

		await done(latest);
	}
}

async function cargoBin(state: CargoState) {
	await cargoImpl({
		state,
		group: 'cargo-bin',
		path: 'Cargo.toml',
		section: 'workspace.metadata.bin',
	});
}

async function dprint() {
	const { dep } = group('dprint');

	const path = '.config/dprint.json';
	const pattern =
		/https:\/\/plugins\.dprint\.dev\/(?<name>\w+)-(?<version>(?:\d+\.){2}\d+)\.wasm/;

	const file = Bun.file(path);
	const contents = await file.text();
	const lines = contents.split('\n');
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
			await Bun.write(file, lines.join('\n'));
		}

		await done(latest);
	}
}

async function npm() {
	await npmImpl('workspace');

	const { workspaces } = await import('../package.json');
	for (const dir of workspaces) {
		chdir(`${repoRoot}/${dir}`);
		await npmImpl(dir);
	}

	chdir(repoRoot);
	await $`rm bun.lockb && bun install`.quiet();
	await commit('Recreate bun.lockb');
	console.info('Recreated bun.lockb');
}

async function npmImpl(name: string) {
	const kinds = [
		[`npm(${name})`, 'dependencies', ''],
		[`npm(${name} dev)`, 'devDependencies', '--dev'],
	] as const;
	const packageJson = async () =>
		JSON.parse(await Bun.file('package.json').text()) as Record<
			(typeof kinds)[number][1],
			Record<string, string>
		>;

	const data = await packageJson();
	for (const [groupName, kind, kindFlag] of kinds) {
		if (data[kind] == null) {
			continue;
		}

		const { dep } = group(groupName);
		for (const [name, current] of Object.entries(data[kind])) {
			const rawCurrent = rawVersion(current);
			const done = dep(name, rawCurrent);

			await $`bun add --exact ${kindFlag} ${name} && bun run biome format --write package.json && git restore ${repoRoot}/bun.lockb`.quiet();

			const newData = await packageJson();
			const latest = newData[kind][name] as string;

			if (rawCurrent === latest) {
				await $`git restore package.json`.quiet();
			}

			await done(latest);
		}
	}
}

async function cargo(state: CargoState) {
	await cargoImpl({
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
		await cargoImpl({
			state,
			group: `cargo(${dir})`,
			path: `${dir}/Cargo.toml`,
			section: 'dependencies',
		});
		await cargoImpl({
			state,
			group: `cargo(${dir} build)`,
			path: `${dir}/Cargo.toml`,
			section: 'build-dependencies',
		});
		await cargoImpl({
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

async function cargoImpl(args: {
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

	const file = Bun.file(args.path);
	const contents = await file.text();
	const lines = contents.split('\n');
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
			await Bun.write(file, lines.join('\n'));

			if (args.updateWorkflows) {
				await updateWorkflows(crate, latest);
			}
		}

		await done(latest);
	}
}

async function githubActions() {
	const { dep } = group('actions');
	const pattern =
		/(?<=uses:\s*)(?<owner>[a-zA-Z_.-]+)\/(?<repo>[a-zA-Z_.-]+)@(?<tag>v?\d+(?:\.\d+(?:\.\d+)?)?)(?!\s*#\s*dep:)$/gm;
	type Match = Record<'prefix' | 'owner' | 'repo' | 'tag', string>;

	const versionPattern = /^v?(\d+)(?:\.(\d+)(?:\.(\d+))?)?$/;
	const sortableTag = (tag: string) =>
		tag
			.match(versionPattern)
			?.slice(1)
			?.filter((x) => x != null)
			?.map((x) => Number.parseInt(x));

	const deps = new Map<string, Set<string>>();
	await forAllWorkflows(async (path) => {
		const contents = await Bun.file(path).text();
		for (const match of contents.matchAll(pattern)) {
			const { owner, repo, tag } = match.groups as Match;
			const key = `${owner}\0${repo}\0${tag}`;
			if (!deps.has(key)) {
				deps.set(key, new Set());
			}
			deps.get(key)!.add(path);
		}
	});

	for (const [key, paths] of [...deps].sort()) {
		const [owner, repo, oldTag] = key.split('\0');
		const name = `${owner}/${repo}`;
		const done = dep(name, oldTag);

		const { data } = await ghRequest(
			'GET /repos/{owner}/{repo}/git/matching-refs/{ref}',
			{
				owner,
				repo,
				ref: 'tags/',
			},
		);
		type SortableTags = Array<[Array<number>, string]>;
		const sortableTags = data
			.map(({ ref }) => ref.slice(10))
			.map((tag) => [sortableTag(tag), tag])
			.filter(([match]) => match != null) as SortableTags;
		const [, latestTag] = sortableTags.reduce(
			(best, next) => {
				const length = Math.max(best[0].length, next[0].length);
				for (let i = 0; i <= length; i++) {
					const digitBest = best[0][i];
					const digitNext = next[0][i];
					if (digitNext > digitBest || digitBest == null) {
						return next;
					}
					if (digitNext < digitBest) {
						return best;
					}
				}
				return best;
			},
			[sortableTag(oldTag) ?? [], oldTag],
		);

		const pattern = new RegExp(
			`(?<=uses:\\s*${escapeRegExp(name)}@)${escapeRegExp(oldTag)}`,
			'g',
		);
		await Promise.all([
			[...paths].map(updateFile((s) => s.replaceAll(pattern, latestTag))),
		]);

		await done(latestTag);
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
		async updateWorkflows(dep: string, latest: string) {
			const pattern = new RegExp(
				`(?<=(?:@|:\\s*)v?)(\\d+(?:\\.\\d+(?:\\.\\d+)))(?=\\s*#\\s*dep:${escapeRegExp(group)}:${escapeRegExp(dep)}\\s*$)`,
				'gm',
			);
			await forAllWorkflows(
				updateFile((s) => s.replaceAll(pattern, latest)),
			);
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

async function forAllWorkflows(
	callback: (path: string) => Promise<void> | void,
) {
	for (const [dir, glob] of [
		['workflows', '*.yaml'],
		['actions', '*/*.yaml'],
	]) {
		const cwd = `${repoRoot}/.github/${dir}`;
		for await (const path of new Glob(glob).scan({ cwd, absolute: true })) {
			await callback(path);
		}
	}
}

function updateFile(
	update: (contents: string) => string,
): (path: string) => Promise<void> {
	return async (path) => {
		const file = Bun.file(path);
		const contents = update(await file.text());
		await Bun.write(file, contents);
	};
}

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions#escaping
function escapeRegExp(s: string) {
	return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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
