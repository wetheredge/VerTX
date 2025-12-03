import { exec, orExit } from './cli.ts';

type CargoBuild = {
	command?: string;
	package?: string;
	buildStd?: string | false;
	target: string;
	features?: Array<string>;
	release?: boolean;
	extraArgs?: Array<string>;
	env?: Record<string, string>;
	cwd?: string;
};
export function cargoBuild(opts: CargoBuild): Promise<void> {
	return orExit(
		exec(
			'cargo',
			opts.command ?? 'build',
			`--package=${opts.package ?? 'vertx'}`,
			...(opts.buildStd === false
				? []
				: [`-Zbuild-std=${opts.buildStd}`]),
			`--target=${opts.target}`,
			`--features=${(opts.features ?? []).join(',')}`,
			...(opts.release ? ['--release'] : []),
			...(opts.extraArgs ?? []),
			{
				cwd: opts.cwd,
				env: {
					// biome-ignore lint/style/useNamingConvention: env var
					CARGO_TERM_COLOR: 'always',
					...opts.env,
				},
			},
		),
	);
}
