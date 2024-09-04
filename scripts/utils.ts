import { $ } from 'bun';

export const getRepoRoot = () =>
	$`git rev-parse --show-toplevel`.text().then((s) => s.trim());
