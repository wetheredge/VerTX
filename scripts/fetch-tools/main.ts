import { join } from 'node:path';
import { Listr } from 'listr2';
import { getRepoRoot } from '../utils.ts';
import cargoBins from './cargo-bins.ts';
import rust from './rust.ts';

const repoRoot = await getRepoRoot();
const context = {
	outDir: join(repoRoot, '.tools'),
	downloadDir: join(repoRoot, '.cache/downloads'),
};

export type Context = typeof context;

await new Listr([rust(context), cargoBins(context)]).run();
