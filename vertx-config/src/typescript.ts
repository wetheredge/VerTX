import type * as types from './types';
import { type NonEmptyPath, getWriter, toPascalCase, visit } from './utilities';

export async function typescript(config: types.Config, outFile: string) {
	const { writer, out, outln } = getWriter(Bun.file(outFile));

	const integerToPostcard = ({ raw }: { raw: types.RawInteger }) =>
		['u8', 'i8'].includes(raw)
			? raw
			: raw.startsWith('u')
				? 'varuint'
				: 'varint';

	outln`import type { Reader, Writer } from "postcard";\n`;

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
		enumeration: (_path, { name }) => readType('varuint', name),
		boolean: () => readType('boolean'),
	});
	outln`\t];`;
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

	outln`export type Update = `;
	outln`\t| { key: StringSettings, value: string }`;
	outln`\t| { key: IntegerSettings | EnumSettings, value: number }`;
	outln`\t| { key: BooleanSettings, value: boolean };\n`;

	const updateTypes = ['string', 'u8', 'i8', 'varint', 'varuint', 'boolean'];
	const updates: Record<string, Array<number>> = Object.fromEntries(
		updateTypes.map((key) => [key, []]),
	);
	visit(config, {
		string: (_p, _v, i) => updates.string.push(i),
		integer: (_p, v, i) => updates[integerToPostcard(v)].push(i),
		enumeration: (_p, _v, i) => updates.varuint.push(i),
		boolean: (_p, _v, i) => updates.boolean.push(i),
	});
	outln`export function encodeUpdate(writer: Writer, update:Update): ArrayBuffer {`;
	outln`writer.varuint(update.key);`;
	outln`switch (update.key) {`;
	for (const [type, keys] of Object.entries(updates)) {
		for (const i of keys) {
			outln`case ${i}:`;
		}
		if (keys.length > 0) {
			outln`writer.${type}(update.value); break;`;
		}
	}
	outln`}`;
	outln`return writer.done();`;
	outln`}`;

	await writer.end();
}
