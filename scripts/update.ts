#!/usr/bin/env bun

import { mkdtemp, rm } from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { chdir, stdout } from 'node:process';
import { request as ghRequest } from '@octokit/request';
import { $, type BunFile, type FileSink, Glob } from 'bun';
import { panic, repoRoot } from './utils.ts';

const missingTools = ['mise', 'bun', 'cargo', 'git'].filter(
	(tool) => Bun.which(tool) == null,
);
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

const branchExists = await $`git rev-parse --verify --quiet ${branch}`
	.nothrow()
	.quiet();
if (branchExists.exitCode === 0) {
	panic(`Branch ${branch} already exists`);
}

let prBody: FileSink | null = null;
if (import.meta.env.CI) {
	prBody = Bun.file(`${repoRoot}/.updates.md`).writer();
}

chdir(repoRoot);
await $`git switch -c updates`.quiet();
await mise();
await dprint();
await npm();
await cargo();
await githubActions();

await prBody?.end?.();

async function mutateFileLines(
	file: BunFile,
	cb: (
		line: string,
		replaceLine: (line: string) => Promise<void>,
	) => Promise<void> | Promise<undefined | boolean>,
) {
	const contents = await file.text();
	const lines = contents.split('\n');
	for (let i = 0; i < lines.length; i++) {
		const done = await cb(lines[i]!, async (line) => {
			lines[i] = line;
			await Bun.write(file, lines.join('\n'));
		});

		if (done) {
			break;
		}
	}
}

async function mise() {
	const { dep, updateWorkflows } = group('mise');

	type Install = {
		version: string;
		source?: {
			path?: string;
		};
	};

	const newLocal = $`mise ls --json`;
	const tools: Record<string, Array<Install>> = await newLocal.json();
	for (const [tool, installs] of Object.entries(tools)) {
		const inRepo = installs.find(({ source }) => {
			if (source?.path) {
				const repoRel = path.relative(repoRoot, source.path);
				return !(repoRel.startsWith('..') || path.isAbsolute(repoRel));
			}
		});

		if (inRepo) {
			const current = inRepo.version;
			const done = dep(tool, current);

			let latest: string | undefined;
			if (current.includes('-')) {
				latest = (await $`mise latest ${tool}`.text()).trim();
			} else {
				const all = await $`mise ls-remote ${tool}`.text();
				const isStable = (v: string) => !v.includes('-');
				latest = all.trim().split('\n').findLast(isStable);
			}

			if (!latest) {
				console.warn(`Failed to get latest version for ${tool}`);
			} else if (current !== latest) {
				await Promise.all([
					$`mise use ${tool}@${latest}`.quiet(),
					updateWorkflows(tool, latest),
				]);
			}

			await done(latest ?? current);
		}
	}
}

async function dprint() {
	const { dep } = group('dprint');

	const path = '.config/dprint.json';
	const pattern =
		/https:\/\/plugins\.dprint\.dev\/(?<name>\w+)-(?<version>(?:\d+\.){2}\d+)\.wasm/;

	const file = Bun.file(path);
	mutateFileLines(file, async (line, replaceLine) => {
		const matches = line.match(pattern);
		if (!matches) {
			return;
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
			await replaceLine(line.replace(pattern, response.url));
		}

		await done(latest);
	});
}

async function npm() {
	await npmImpl('workspace');

	const { workspaces } = await import('../package.json');
	for (const dir of workspaces) {
		chdir(`${repoRoot}/${dir}`);
		await npmImpl(dir);
	}

	chdir(repoRoot);
	await rm('bun.lock');
	await $`bun install --save-text-lockfile`.quiet();
	await commit('Recreate bun.lock');
	console.info('Recreated bun.lock');
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
			if (current.startsWith('workspace:')) {
				continue;
			}

			const rawCurrent = rawVersion(current);
			const done = dep(name, rawCurrent);

			await $`bun add --exact ${kindFlag} ${name} && bun run biome format --write package.json && git restore ${repoRoot}/bun.lock`.quiet();

			const newData = await packageJson();
			const latest = newData[kind][name] as string;

			if (rawCurrent === latest) {
				await $`git restore package.json`.quiet();
			}

			await done(latest);
		}
	}
}

async function cargo() {
	const tempCrateDir = await mkdtemp(path.join(os.tmpdir(), 'rust-updates-'));
	await $`cargo init --vcs none --name updates ${tempCrateDir}`.quiet();

	const tempManifest = `${tempCrateDir}/Cargo.toml`;
	const tempManifestLastLine = async () => {
		const contents = await Bun.file(tempManifest).text();
		return contents.trim().split('\n').at(-1) ?? '';
	};

	if ((await tempManifestLastLine()) !== '[dependencies]') {
		panic("Generated Cargo.toml doesn't end with [dependencies] table");
	}

	const getLatest = async (dep: string) => {
		await $`cargo add ${dep}`.cwd(tempCrateDir).quiet();
		const metadata =
			await $`cargo metadata --no-deps --frozen --format-version 1`.json();
		const latest: string = metadata.packages[0].dependencies[0].req;
		await $`cargo rm ${dep}`.quiet();
		return latest.replace(/^[~^=]?/, '=');
	};

	await cargoImpl({
		getLatest,
		group: 'cargo(workspace)',
		path: 'Cargo.toml',
		section: 'workspace.dependencies',
	});

	const metadata =
		await $`cargo metadata --no-deps --format-version 1`.json();
	const members = metadata.packages.map((p: { name: string }) => p.name);

	for (const dir of members) {
		await cargoImpl({
			getLatest,
			group: `cargo(${dir})`,
			path: `${dir}/Cargo.toml`,
			section: 'dependencies',
		});
		await cargoImpl({
			getLatest,
			group: `cargo(${dir} build)`,
			path: `${dir}/Cargo.toml`,
			section: 'build-dependencies',
		});
		await cargoImpl({
			getLatest,
			group: `cargo(${dir} dev)`,
			path: `${dir}/Cargo.toml`,
			section: 'dev-dependencies',
		});
		await cargoImpl({
			getLatest,
			group: `cargo(${dir} wasm)`,
			path: `${dir}/Cargo.toml`,
			section: 'target.wasm32-unknown-unknown.dependencies',
		});
	}

	const generateLockfile = await $`cargo generate-lockfile`.quiet().nothrow();
	if (generateLockfile.exitCode !== 0) {
		await commit('Recreate Cargo.lock');
		console.info('Recreated Cargo.lock');
	} else {
		await $`git restore Cargo.lock`.quiet().nothrow();
		console.warn(
			'Failed to regenerate lockfile! Fix the issue and recreate it manually',
		);
	}

	await rm(tempCrateDir, { recursive: true });
}

async function cargoImpl(args: {
	getLatest: (dep: string) => Promise<string>;
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
	let foundSection = false;
	await mutateFileLines(file, async (rawLine, replaceLine) => {
		const [line, comment] = splitComment(rawLine, commentPattern);

		if (!foundSection) {
			if (line.replace(/\s/, '') === `[${args.section}]`) {
				foundSection = true;
			} else {
				return;
			}
		}

		if (line.trim().startsWith('[')) {
			return true;
		}

		const matches = line.match(depPattern)?.slice(1);
		if (!matches) {
			return;
		}
		const crate = matches[crateId]!;
		const current = matches[versionId]!;
		const rawCurrent = rawVersion(current);

		const done = dep(crate, rawCurrent);
		const latest = await args.getLatest(crate);

		if (rawCurrent !== latest) {
			await Promise.all([
				replaceLine(matches.with(versionId, latest).join('') + comment),
				args.updateWorkflows && updateWorkflows(crate, latest),
			]);
		}

		await done(latest);
	});
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
		type SplitKey = [string, string, string];
		const [owner, repo, oldTag] = key.split('\0') as SplitKey;
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
					const digitBest = best[0][i]!;
					const digitNext = next[0][i]!;
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

					if (prBody) {
						if (!updatedAny) {
							prBody.write(
								`### \`${group}\`\ndependency|from|to\n-|-|-\n`,
							);
							updatedAny = true;
						}
						prBody.write(`\`${dep}\`|${current}|${latest}\n`);
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
	] as const) {
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

// FIXME: inline once TypeScript knows about RegExp.escape
function escapeRegExp(s: string) {
	// @ts-ignore
	return RegExp.escape(s);
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
