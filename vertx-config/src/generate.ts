import * as old from '../config.old.ts';
import * as current from '../config.ts';
import { rust } from './rust.ts';
import { typescript } from './typescript.ts';

await Promise.all([
	rust(current, 'out/config.rs'),
	rust(current, 'out/current.rs', true),
	rust(old, 'out/old.rs', true),
	typescript(current, 'out/config.ts'),
]);
