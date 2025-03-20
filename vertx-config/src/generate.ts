import * as current from '../config';
import * as old from '../config.old';
import { rust } from './rust';
import { typescript } from './typescript';

await Promise.all([
	rust(current, 'out/config.rs'),
	rust(current, 'out/current.rs', true),
	rust(old, 'out/old.rs', true),
	typescript(current, 'out/config.ts'),
]);
