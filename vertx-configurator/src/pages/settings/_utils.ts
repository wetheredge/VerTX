import type { Config } from '../../config.ts';

export type SettingProps<Key extends keyof Config = keyof Config> = {
	key: Key;
	containerId?: string;
	label: string;
	description?: string;
};

type ToString = { toString(): string };

export function getId(key: ToString): string {
	return `s${key}`;
}

export function getDescriptionId(key: ToString): string {
	return `d${key}`;
}

export function split<T extends Record<string, unknown>, K extends keyof T>(
	obj: T,
	keys: Array<K>,
): [Pick<T, K>, Omit<T, K>] {
	const left = {} as Pick<T, K>;
	const right = { ...obj } as Omit<T, K>;

	for (const key of keys) {
		if (key in right) {
			left[key] = obj[key];
			// @ts-expect-error
			delete right[key];
		}
	}

	return [left, right];
}
