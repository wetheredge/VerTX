import { argv, env, exit } from 'node:process';
import { fileURLToPath } from 'node:url';
import spawn, {
	type Options as SpawnOptions,
	type Subprocess,
	SubprocessError,
} from 'nano-spawn';

export function isMain(importMetaUrl: string): boolean {
	return argv[1] === fileURLToPath(importMetaUrl);
}

export function panic(message: string): never {
	console.error(message);
	if (env.CI) {
		console.log(`::error::${message}\n`);
	}
	exit(1);
}

type ExecArgs = Array<string | null>;
type ExecOptions = SpawnOptions & { quiet?: boolean };
export function exec(
	cmd: string,
	...args: ExecArgs | [...ExecArgs, ExecOptions]
): Subprocess {
	const maybeOpts = args.at(-1);
	let opts: ExecOptions = {};
	if (typeof maybeOpts === 'object' && maybeOpts != null) {
		opts = args.pop() as ExecOptions;
	}
	const { quiet, ...spawnOpts } = opts;

	return spawn(
		cmd,
		(args as ExecArgs).filter((x) => typeof x === 'string'),
		{
			preferLocal: true,
			stdout: quiet ? 'ignore' : 'inherit',
			stderr: quiet ? 'ignore' : 'inherit',
			...spawnOpts,
		},
	);
}

export async function orExit(child: Subprocess) {
	try {
		await child;
	} catch (err) {
		if (err instanceof SubprocessError && err.exitCode != null) {
			exit(err.exitCode);
		}
		throw err;
	}
}
