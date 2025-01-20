import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { Listr } from 'listr2';
import { getRepoRoot } from '../utils.ts';
import cargoBins from './cargo-bins.ts';
import { gcc, rust } from './rust.ts';

const repoRoot = await getRepoRoot();
const context = {
	outDir: join(repoRoot, '.tools'),
	downloadDir: join(repoRoot, '.cache/downloads'),
};

export type Context = typeof context;

await mkdir(context.outDir, { recursive: true });
await new Listr([rust(context), gcc(context), cargoBins(context)]).run();
