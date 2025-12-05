import { Utf8Stream } from 'node:fs';
import * as types from './types.ts';

export type ConfigMeta = {
	config: types.Config;
	version: number;
};

type Out = (
	s: TemplateStringsArray,
	...args: Array<{ toString(): string }>
) => void;
export function getWriter(path: string): {
	stream: Utf8Stream;
	out: Out;
	outln: Out;
} {
	const stream = new Utf8Stream({
		append: false,
		dest: path,
		sync: true,
	});

	const out: Out = (segments, ...args) =>
		segments.forEach((s, i) => {
			stream.write(s);
			if (args[i] != null) {
				stream.write(args[i].toString());
			}
		});

	const outln: Out = (segments, ...args) => {
		out(segments, ...args);
		stream.write('\n');
	};

	return { stream, out, outln };
}

export type Path = Array<string>;
export type NonEmptyPath = [...Array<string>, string];
type Visitor = {
	[Key in keyof types.Leaf]?: (
		path: NonEmptyPath,
		value: types.Leaf[Key],
		index: number,
	) => void;
} & {
	leaf?: (path: NonEmptyPath, index: number) => void;
	startNested?: (
		path: Path,
		keys: Array<{ key: string; isLeaf: boolean }>,
	) => void;
	endNested?: (path: Path) => void;
};
export function visit(config: types.Config, visitor: Visitor) {
	const getKeys = (value: types.Config) =>
		Object.entries(value).map(([key, value]) => ({
			key,
			isLeaf: value.type != null,
		}));
	const visitLeaf = (path: NonEmptyPath, _: unknown, i: number) =>
		visitor.leaf?.(path, i);
	const visitString = visitor.string ?? visitLeaf;
	const visitInteger = visitor.integer ?? visitLeaf;
	const visitEnumeration = visitor.enumeration ?? visitLeaf;
	const visitBoolean = visitor.boolean ?? visitLeaf;

	let i = 0;
	const impl = (config: types.Config, parent: Path = []) => {
		for (const [key, value] of Object.entries(config)) {
			const path: NonEmptyPath = [...parent, key];
			if (value.type == null) {
				const keys = getKeys(value);
				visitor.startNested?.(path, keys);
				impl(value, path);
				visitor.endNested?.(path);
			} else if (types.isString(value)) {
				visitString(path, value, i++);
			} else if (types.isInteger(value)) {
				visitInteger(path, value, i++);
			} else if (types.isEnumeration(value)) {
				visitEnumeration(path, value, i++);
			} else if (types.isBoolean(value)) {
				visitBoolean(path, value, i++);
			} else {
				unreachable(value);
			}
		}
	};

	visitor.startNested?.([], getKeys(config));
	impl(config);
	visitor.endNested?.([]);
}

function splitCamelCase(camel: string): Array<string> {
	return camel.split(/(?=[A-Z])/);
}

export function toPascalCase(camel: string): string {
	return splitCamelCase(camel)
		.map((x) => {
			// biome-ignore lint/style/noNonNullAssertion: splitCamelCase won't return empty strings
			return x[0]!.toUpperCase() + x.substring(1);
		})
		.join('');
}

export function toSnakeCase(camel: string): string {
	return splitCamelCase(camel)
		.map((x) => x.toLowerCase())
		.join('_');
}

export function byteLength(config: types.Config): number {
	const varint = { x8: 1, x16: 3, x32: 5, x64: 10, x128: 19 };
	const usize = varint.x32;

	let length = 4; // u32 version
	visit(config, {
		string(_, str) {
			length += usize + str.length;
		},
		integer(_, { raw }) {
			length += varint[raw.replace(/^[iu]/, 'x') as keyof typeof varint];
		},
		enumeration() {
			length += usize;
		},
		boolean() {
			length += 1;
		},
	});
	return length;
}

function unreachable(x: never): never {
	throw new Error('Reached unreachable: ', x);
}
