import {
	byteLength,
	type ConfigMeta,
	getWriter,
	type Path,
	toPascalCase,
	toSnakeCase,
	visit,
} from './utilities.ts';

export async function rust(
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
	outln`#[derive(Debug, ::serde::Deserialize, ::serde::Serialize)]`;
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

	outln`#[allow(clippy::derivable_impls)]`;
	outln`impl Default for RawConfig {`;
	outln`    fn default() -> Self {`;
	outln`        Self {`;
	const fieldDefault = (path: Path, def: { toString(): string }) =>
		outln`${getFieldName(path)}: ${def},`;
	visit(config, {
		string: (path, { def }) =>
			fieldDefault(
				path,
				def == null
					? 'Default::default()'
					: `"${def}".try_into().unwrap()`,
			),
		integer: (path, { def }) => fieldDefault(path, def),
		enumeration: (path) => fieldDefault(path, 'Default::default()'),
		boolean: (path, { def }) => fieldDefault(path, def),
	});
	outln`        }`;
	outln`    }`;
	outln`}`;

	out`pub(crate) const BYTE_LENGTH: usize = ${byteLength(config)};\n`;

	// Enum declarations
	visit(config, {
		enumeration(_path, value) {
			outln`#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ::serde::Deserialize, ::serde::Serialize)]`;
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
		outln`pub(super) fn diff(&self, other: &Self, mut different: impl FnMut(usize)) {`;
		visit(config, {
			leaf(path, i) {
				const field = getFieldName(path);
				outln`    if self.${field} != other.${field} {`;
				outln`        different(${i});`;
				outln`    }`;
			},
		});
		outln`}`;
	}

	outln`}`;

	await writer.end();
}
