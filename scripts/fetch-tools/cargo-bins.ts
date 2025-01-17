import { $ } from 'bun';
import type { Listr, ListrTask } from 'listr2';
// @ts-ignore
import { workspace } from '../../Cargo.toml';
import type { Context } from './main';

type Dependencies = Array<[string, string]>;

const dependencies = Object.entries(
	workspace.metadata['bin-dependencies'],
) as Dependencies;

export default (context: Context): ListrTask => ({
	title: 'cargo install',
	async task(_ctx, task): Promise<Listr> {
		const tryBinstall = await $`cargo binstall -V`.quiet().nothrow();

		const subtasks: Array<ListrTask> = dependencies.map(
			([name, rawVersion]) => {
				const version = rawVersion.replace(/^=/, '');
				return {
					title: `${name} v${version.replace(/^=/, '')}`,
					async task() {
						const spec = `${name}@${version}`;
						const root = `--root=${context.outDir}`;
						if (tryBinstall.exitCode === 0) {
							await $`cargo binstall ${spec} --locked ${root} --no-confirm`.quiet();
						} else {
							await $`cargo install ${spec} --locked ${root}`.quiet();
						}
					},
				};
			},
		);

		return task.newListr(subtasks);
	},
});
