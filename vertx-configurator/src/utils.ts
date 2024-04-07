import type { ResponseKind, ResponsePayload } from './api';

export function formatVersion(
	build?: ResponsePayload<ResponseKind.BuildInfo>,
): string | undefined {
	if (!build) {
		return;
	}

	// Unicode hyphen to avoid Inter's tnum feature making it into a minus sign
	const suffix = build.suffix ? `\u{2011}${build.suffix}` : '';
	return `v${build.major}.${build.minor}.${build.patch}${suffix}`;
}

export function unreachable(value: never): never {
	throw new Error(`Unreachable: ${value}`);
}
