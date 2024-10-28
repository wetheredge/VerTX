import { ResponseKind } from './api';
import { api } from './api';

export const isSimulator = import.meta.env.VITE_TARGET === 'simulator';

export function version(): string | undefined {
	const build = api[ResponseKind.BuildInfo];
	if (!build) {
		return;
	}

	// Unicode hyphen to avoid Inter's tnum feature making it into a minus sign
	return `v${build.version}`.replaceAll('-', '\u{2011}');
}

export function unreachable(value: never): never {
	throw new Error(`Unreachable: ${value}`);
}
