import { chdir } from 'node:process';
import { nightly, chips as rawChips } from '../.config/toolchains.json';
import { getRepoRoot } from './utils';

// Consistent paths
chdir(await getRepoRoot());

const chips = mapValues(rawChips, ({ toolchain, ...rest }) => ({
	...rest,
	toolchain: toolchain === '$nightly' ? nightly : toolchain,
}));
const chipsStr = JSON.stringify(chips);

const taskfiles = ['Taskfile.yaml', '.config/tasks/Taskfile.embedded.yaml'];
for (const path of taskfiles) {
	const nightlyPrefix = '  nightly:';
	const chipsPrefix = '  raw_chip_data:';

	const file = Bun.file(path);
	const contents = await file.text();
	const updated = contents
		.split('\n')
		.map((line) =>
			line.startsWith(nightlyPrefix)
				? `${nightlyPrefix} ${nightly}`
				: line.startsWith(chipsPrefix)
					? `${chipsPrefix} '${chipsStr}'`
					: line,
		)
		.join('\n');
	Bun.write(file, updated);
}

const workflows = ['ci'];
for (const workflow of workflows) {
	const prefix = '  NIGHTLY:';

	const file = Bun.file(`.github/workflows/${workflow}.yaml`);
	const contents = await file.text();
	const updated = mapLines(contents, (line) =>
		line.startsWith(prefix) ? `${prefix} ${nightly}` : line,
	);
	Bun.write(file, updated);
}

function mapValues<T, U>(
	obj: Record<string, T>,
	map: (value: T) => U,
): Record<string, U> {
	return Object.fromEntries(
		Object.entries(obj).map(([key, value]) => [key, map(value)]),
	);
}

function mapLines(s: string, map: (line: string) => string): string {
	return s.split('\n').map(map).join('\n');
}
