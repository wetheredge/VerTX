export type Config = { [key: string]: ConfigValue } & { type?: never };
export type FlatConfigValue = Leaf[keyof Leaf];
export type ConfigValue = FlatConfigValue | Config;

const typeString = Symbol('string');
export const string = (length: number, def?: string) => ({
	type: typeString,
	length,
	def,
});
export const isString = (
	obj: Record<string, unknown>,
): obj is ReturnType<typeof string> => obj.type === typeString;

const typeInteger = Symbol('integer');
export type RawInteger = `${'i' | 'u'}${'8' | '16' | '32'}`;
export const integer = (
	type: RawInteger,
	def: number,
	{ min, max }: { min?: number; max?: number },
) => ({ type: typeInteger, raw: type, def, min, max });
export const isInteger = (
	obj: Record<string, unknown>,
): obj is ReturnType<typeof integer> => obj.type === typeInteger;

const typeEnumeration = Symbol('enum');
export const enumeration = (
	name: string,
	variants: Array<{ name: string; ident?: string; default?: boolean }>,
) => {
	if (variants.filter((v) => v.default).length !== 1) {
		throw new Error('Exactly 1 variant must be marked the default');
	}
	return {
		type: typeEnumeration,
		name,
		variants: variants.map((variant) => ({
			name: variant.name,
			ident: variant.ident ?? variant.name,
			default: Boolean(variant.default),
		})),
	};
};
export const isEnumeration = (
	obj: Record<string, unknown>,
): obj is ReturnType<typeof enumeration> => obj.type === typeEnumeration;

const typeBoolean = Symbol('boolean');
export const boolean = (def = false) => ({ type: typeBoolean, def });
export const isBoolean = (
	obj: Record<string, unknown>,
): obj is ReturnType<typeof boolean> => obj.type === typeBoolean;

const rawLeafFns = { string, integer, enumeration, boolean } as const;
type LeafFns = typeof rawLeafFns;

export type Leaf = {
	[Key in keyof LeafFns]: ReturnType<LeafFns[Key]>;
};
