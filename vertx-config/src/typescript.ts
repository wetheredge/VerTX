import type * as types from './types';
import {
	type ConfigMeta,
	type NonEmptyPath,
	byteLength,
	getWriter,
	toPascalCase,
	visit,
} from './utilities';

export async function typescript(
	{ config, version }: ConfigMeta,
	outFile: string,
) {
	const { writer, out, outln } = getWriter(Bun.file(outFile));

	const integerToPostcard = (raw: types.RawInteger) =>
		['u8', 'i8'].includes(raw)
			? raw
			: raw.startsWith('u')
				? 'varuint'
				: 'varint';

	outln`import { type Reader, Writer } from "postcard";\n`;

	outln`export const configKeys = {`;
	visit(config, {
		// biome-ignore lint/style/noNonNullAssertion: is known to be non-empty
		leaf: (path: NonEmptyPath, i: number) => outln`${path.at(-1)!}: ${i},`,
		// biome-ignore lint/style/noNonNullAssertion: checked first
		startNested: (path) => path.length > 0 && out`${path.at(-1)!}: {\n`,
		endNested: (path) => path.length > 0 && outln`},`,
	});
	outln`} as const;\n`;

	outln`export type Config = [`;
	const type = (type: string) => outln`\t${type},`;
	visit(config, {
		string: () => type('string'),
		integer: () => type('number'),
		enumeration: (_path, { name }) => type(name),
		boolean: () => type('boolean'),
	});
	outln`];\n`;

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
	outln`\t// Ignore u32 version`;
	outln`\tfor (let i = 0; i < 4; i++) { reader.u8() }`;
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
		integer: (_path, { raw }) => readType(integerToPostcard(raw)),
		enumeration: (_path, { name }) => readType('varuint', name),
		boolean: () => readType('boolean'),
	});
	outln`\t];`;
	outln`}\n`;

	outln`export function encodeConfig(config: Config): ArrayBuffer {`;
	outln`\tconst writer = new Writer(${byteLength(config)});`;
	outln`\twriter.rawU32(${version});`;
	const writeField = (i: number, type: string, as?: string) => {
		out`\twriter.${type}(config[${i}]`;
		if (as != null) {
			out` as ${as}`;
		}
		outln`);`;
	};
	visit(config, {
		string: (_p, _v, i) => writeField(i, 'string'),
		integer: (_p, { raw }, i) => writeField(i, integerToPostcard(raw)),
		enumeration: (_p, _v, i) => writeField(i, 'varuint', 'number'),
		boolean: (_p, _v, i) => writeField(i, 'boolean'),
	});
	outln`\treturn writer.done();`;
	outln`}\n`;

	const settingsByType: Record<
		'string' | 'integer' | 'enum' | 'boolean',
		Array<number>
	> = {
		string: [],
		integer: [],
		enum: [],
		boolean: [],
	};
	visit(config, {
		string: (_p, _v, i) => settingsByType.string.push(i),
		integer: (_p, _v, i) => settingsByType.integer.push(i),
		enumeration: (_p, _v, i) => settingsByType.enum.push(i),
		boolean: (_p, _v, i) => settingsByType.boolean.push(i),
	});
	for (const [type, indices] of Object.entries(settingsByType)) {
		out`export type ${toPascalCase(type)}Settings = `;
		if (indices.length > 0) {
			outln`${indices.join(' | ')};`;
		} else {
			outln`never;`;
		}
	}
	outln``;

	await writer.end();
}
