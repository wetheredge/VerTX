import type { BunFile, FileSink } from 'bun';
import * as current from './config';
import * as old from './config.old';
import * as types from './types';

await Promise.all([
	rust(current, 'out/config.rs'),
	rust(current, 'out/current.rs', true),
	rust(old, 'out/old.rs', true),
	typescript(current.config, 'out/config.ts'),
]);

type ConfigMeta = {
	config: types.Config;
	version: number;
};

async function rust(
	{ config, version }: ConfigMeta,
	outFile: string,
	migration = false,
) {
	const { writer, out, outln } = getWriter(Bun.file(outFile));

	const string = ({ length }: { length: number }) =>
		`::heapless::String<${length}>`;
	const getFieldName = (path: Path) => path.join('_');
	const getRawKeyType = (path: Path) =>
		['Root', ...path.map(toPascalCase)].join('_');
	const getKeyType = (path: Path) => `key::${getRawKeyType(path)}`;

	// RawConfig struct
	outln`#[derive(Debug, Default, ::serde::Deserialize, ::serde::Serialize)]`;
	outln`#[allow(non_snake_case)]`;
	outln`pub(crate) struct RawConfig {`;
	const rawConfigField = (path: Path, type: string) =>
		outln`pub(super) ${getFieldName(path)}: ${type},`;
	visit(config, {
		string: (path, value) => rawConfigField(path, string(value)),
		integer: (path, { raw }) => rawConfigField(path, raw),
		enumeration: (path, { name }) => rawConfigField(path, name),
		boolean: (path) => rawConfigField(path, 'bool'),
	});
	outln`}\n`;

	// 4 for u32 version bytes
	out`pub(crate) const BYTE_LENGTH: usize = 4`;
	const varintByteLength = { x8: 1, x16: 3, x32: 5, x64: 10, x128: 19 };
	const usizeByteLength = varintByteLength.x32;
	visit(config, {
		string: (_, { length }) => out` + ${length + usizeByteLength}`,
		integer: (_, { raw }) =>
			out` + ${varintByteLength[raw.replace(/^[iu]/, 'x') as keyof typeof varintByteLength]}`,
		enumeration: () => out` + ${usizeByteLength}`,
		boolean: () => out` + 1`,
	});
	outln`;\n`;

	// Enum declarations
	visit(config, {
		enumeration(_path, value) {
			outln`#[derive(Debug, Default, Clone, Copy, ::serde::Deserialize, ::serde::Serialize)]`;
			outln`pub(crate) enum ${value.name} {`;
			for (const variant of value.variants) {
				if (variant.name !== variant.ident) {
					outln`/// ${variant.name}`;
				}
				if (variant.default) {
					outln`#[default]`;
				}
				outln`${variant.ident},`;
			}
			outln`}\n`;
		},
	});

	if (!migration) {
		// Typestate keys
		const typestateKeyStruct = (path: Path) =>
			outln`#[derive(Clone, Copy)] pub(crate) struct ${getRawKeyType(path)};`;
		outln`#[allow(non_camel_case_types, unused)]`;
		outln`pub(super) mod key {`;
		visit(config, {
			leaf: typestateKeyStruct,
			startNested: typestateKeyStruct,
		});
		outln`}\n`;

		// Typestate impls
		const leafImplBlock = (path: Path, type: string, i: number) => {
			const keyType = getKeyType(path);

			outln`#[allow(unused)]`;
			outln`impl super::View<${keyType}> {`;
			outln`    pub(crate) fn lock<T>(&self, f: impl FnOnce(&${type}) -> T) -> T {`;
			outln`        self.manager.state.lock(|state| f(&state.borrow().config.${getFieldName(path)}))`;
			outln`    }\n`;
			outln`    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {`;
			outln`        self.manager.subscribe(${i})`;
			outln`    }`;
			outln`}\n`;

			outln`impl ::core::ops::Deref for super::LockedView<'_, ${keyType}> {`;
			outln`    type Target = ${type};`;
			outln`    fn deref(&self) -> &Self::Target {`;
			outln`        &self.config.${getFieldName(path)}`;
			outln`    }`;
			outln`}\n`;
		};
		visit(config, {
			startNested(path, keys) {
				const keyType = getKeyType(path);

				outln`#[allow(unused)]`;
				outln`impl super::View<${keyType}> {`;
				outln`    pub(crate) fn lock<T>(&self, f: impl FnOnce(super::LockedView<'_, ${keyType}>) -> T) -> T {`;
				outln`        self.manager.state.lock(|state| f(super::LockedView {`;
				outln`            config: &state.borrow().config,`;
				outln`            _key: ::core::marker::PhantomData,`;
				outln`        }))`;
				outln`    }\n`;
				for (const { key } of keys) {
					outln`    pub(crate) fn ${toSnakeCase(key)}(&self) -> super::View<${getKeyType([...path, key])}> {`;
					outln`        super::View { manager: self.manager, _key: ::core::marker::PhantomData }`;
					outln`    }`;
				}
				outln`}\n`;

				outln`#[allow(unused)]`;
				outln`impl super::LockedView<'_, ${keyType}> {`;
				for (const { key } of keys) {
					outln`    pub(crate) fn ${toSnakeCase(key)}(&self) -> super::LockedView<'_, ${getKeyType([...path, key])}> {`;
					outln`        super::LockedView { config: self.config, _key: ::core::marker::PhantomData }`;
					outln`    }`;
				}
				outln`}\n`;
			},
			string: (path, value, i) => leafImplBlock(path, string(value), i),
			integer: (path, { raw }, i) => leafImplBlock(path, raw, i),
			enumeration: (path, { name }, i) => leafImplBlock(path, name, i),
			boolean: (path, _, i) => leafImplBlock(path, 'bool', i),
		});

		// Update enum
		outln`#[derive(Debug, Clone, ::serde::Deserialize)]`;
		outln`#[allow(non_camel_case_types)]`;
		outln`#[serde(tag = "key", content = "value")]`;
		outln`pub(crate) enum Update<'a> {`;
		const updateVariant = (path: Path, type: string) =>
			outln`${getRawKeyType(path)}(${type}),`;
		visit(config, {
			string: (path) => {
				outln`#[serde(borrow)]`;
				updateVariant(path, "&'a str");
			},
			integer: (path, { raw }) => updateVariant(path, raw),
			enumeration: (path, { name }) => updateVariant(path, name),
			boolean: (path) => updateVariant(path, 'bool'),
		});
		outln`}\n`;
	}

	outln`#[derive(Debug, Clone)]`;
	outln`pub(super) enum DeserializeError {`;
	outln`    WrongVersion,`;
	outln`    Postcard(postcard::Error),`;
	outln`}\n`;

	outln`impl RawConfig {`;

	const versionBytes = `u32::to_le_bytes(${version})`;
	outln`pub(super) fn deserialize(from: &[u8]) -> Result<Self, DeserializeError> {`;
	outln`    let (version, from) = from.split_at(4);`;
	outln`    if version == ${versionBytes} {`;
	outln`        postcard::from_bytes(from).map_err(DeserializeError::Postcard)`;
	outln`    } else {`;
	outln`        Err(DeserializeError::WrongVersion)`;
	outln`    }`;
	outln`}`;

	outln`pub(super) fn serialize(&self, buffer: &mut [u8]) -> postcard::Result<usize> {`;
	outln`    let (version, buffer) = buffer.split_at_mut(4);`;
	outln`    version.copy_from_slice(&${versionBytes});`;
	outln`    postcard::to_slice(self, buffer).map(|out| out.len() + 4)`;
	outln`}`;

	if (!migration) {
		outln`pub(super) fn update(&mut self, update: Update<'_>) -> Result<usize, super::UpdateError> {`;
		outln`match update {`;
		const updateArmStart = (path: Path, i: number) => {
			outln`Update::${getRawKeyType(path)}(update) => {`;
			return () => {
				outln`self.${getFieldName(path)} = update;`;
				outln`Ok(${i})`;
				outln`}`;
			};
		};
		visit(config, {
			leaf(path, i) {
				updateArmStart(path, i)();
			},
			string(path, { length }, i) {
				const end = updateArmStart(path, i);
				outln`let Ok(update) = update.try_into() else { return Err(super::UpdateError::TooLarge { max: ${length} }) };`;
				end();
			},
			integer(path, { min, max }, i) {
				const end = updateArmStart(path, i);
				if (min != null) {
					outln`if update < ${min} { return Err(super::UpdateError::TooSmall { min: ${min} }) };`;
				}
				if (max != null) {
					outln`if update > ${max} { return Err(super::UpdateError::TooLarge { max: ${max} }) };`;
				}
				end();
			},
		});
		outln`}`;
		outln`}`;
	}

	outln`}`;

	await writer.end();
}

async function typescript(config: types.Config, outFile: string) {
	const { writer, out, outln } = getWriter(Bun.file(outFile));

	const integerToPostcard = ({ raw }: { raw: types.RawInteger }) =>
		['u8', 'i8'].includes(raw)
			? raw
			: raw.startsWith('u')
				? 'varuint'
				: 'varint';

	outln`import { Reader, Writer } from "postcard";\n`;

	outln`export const configKeys = {`;
	visit(config, {
		// biome-ignore lint/style/noNonNullAssertion: is known to be non-empty
		leaf: (path: NonEmptyPath, i: number) => outln`${path.at(-1)!}: ${i},`,
		// biome-ignore lint/style/noNonNullAssertion: checked first
		startNested: (path) => path.length > 0 && out`${path.at(-1)!}: {\n`,
		endNested: (path) => path.length > 0 && outln`},`,
	});
	outln`} as const;\n`;

	outln`export type Config = {`;
	const type = (type: string, i: number) => outln`\t${i}: ${type}`;
	visit(config, {
		string: (_p, _v, i) => type('string', i),
		integer: (_p, _v, i) => type('number', i),
		enumeration: (_path, { name }, i) => type(name, i),
		boolean: (_p, _v, i) => type('boolean', i),
	});
	outln`};\n`;

	visit(config, {
		enumeration(_path, value) {
			outln`export const enum ${value.name} {`;
			for (const variant of value.variants) {
				outln`${variant.ident},`;
			}
			outln`}\n`;
		},
	});

	outln`export function parseConfig(reader: Reader): Config {`;
	outln`\treturn [`;
	const readType = (type: string, as?: string) => {
		out`\t\treader.${type}()`;
		if (as != null) {
			out` as ${as}`;
		}
		outln`,`;
	};
	visit(config, {
		string: () => readType('string'),
		integer: (_path, value) => readType(integerToPostcard(value)),
		enumeration: (_path, { name }) => readType('varint', name),
		boolean: () => readType('boolean'),
	});
	outln`\t];`;
	outln`}\n`;

	const updateTypes = ['string', 'u8', 'i8', 'varint', 'varuint', 'boolean'];
	const updates: Record<string, Array<number>> = Object.fromEntries(
		updateTypes.map((key) => [key, []]),
	);
	let maxStringLength = 0;
	visit(config, {
		string: (_p, { length }, i) => {
			maxStringLength = Math.max(maxStringLength, length);
			return updates.string.push(i);
		},
		integer: (_p, v, i) => updates[integerToPostcard(v)].push(i),
		enumeration: (_p, _v, i) => updates.varuint.push(i),
		boolean: (_p, _v, i) => updates.boolean.push(i),
	});
	outln`export function update<Key extends keyof Config>(key: Key, value: Config[Key]): ArrayBuffer {`;
	outln`const writer = new Writer(${maxStringLength + 10});`;
	outln`switch (key) {`;
	for (const [type, keys] of Object.entries(updates)) {
		for (const i of keys) {
			outln`case ${i}:`;
		}
		if (keys.length > 0) {
			outln`writer.${type}(value); break;`;
		}
	}
	outln`}`;
	outln`return writer.done();`;
	outln`}`;

	await writer.end();
}

type Out = (
	s: TemplateStringsArray,
	...args: Array<{ toString(): string }>
) => void;
function getWriter(outFile: BunFile): {
	writer: FileSink;
	out: Out;
	outln: Out;
} {
	const writer = outFile.writer();
	const out: Out = (segments, ...args) =>
		segments.forEach((s, i) => {
			writer.write(s);
			if (i < args.length) {
				writer.write(args[i].toString());
			}
		});
	const outln: Out = (segments, ...args) => {
		out(segments, ...args);
		out`\n`;
	};
	return { writer, out, outln };
}

type Path = Array<string>;
type NonEmptyPath = [...Array<string>, string];
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

function toPascalCase(camel: string): string {
	return splitCamelCase(camel)
		.map((x) => x[0].toUpperCase() + x.substring(1))
		.join('');
}

function toSnakeCase(camel: string): string {
	return splitCamelCase(camel)
		.map((x) => x.toLowerCase())
		.join('_');
}

function unreachable(x: never): never {
	throw new Error('Reached unreachable: ', x);
}
