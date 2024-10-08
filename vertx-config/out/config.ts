import type { Reader, Writer } from 'postcard';

export const configKeys = {
	name: 0,
	leds: {
		brightness: 1,
	},
	display: {
		brightness: 2,
		fontSize: 3,
	},
	network: {
		hostname: 4,
		password: 5,
		home: {
			ssid: 6,
			password: 7,
		},
	},
	expert: 8,
} as const;

export type Config = {
	0: string;
	1: number;
	2: number;
	3: FontSize;
	4: string;
	5: string;
	6: string;
	7: string;
	8: boolean;
};

export const enum FontSize {
	Size7px,
	Size9px,
}

export function parseConfig(reader: Reader): Config {
	return [
		reader.string(),
		reader.u8(),
		reader.u8(),
		reader.varuint() as FontSize,
		reader.string(),
		reader.string(),
		reader.string(),
		reader.string(),
		reader.boolean(),
	];
}

export type StringSettings = 0 | 4 | 5 | 6 | 7;
export type IntegerSettings = 1 | 2;
export type EnumSettings = 3;
export type BooleanSettings = 8;

export type Update =
	| { key: StringSettings; value: string }
	| { key: IntegerSettings | EnumSettings; value: number }
	| { key: BooleanSettings; value: boolean };

export function encodeUpdate(writer: Writer, update: Update): ArrayBuffer {
	writer.varuint(update.key);
	switch (update.key) {
		case 0:
		case 4:
		case 5:
		case 6:
		case 7:
			writer.string(update.value);
			break;
		case 1:
		case 2:
			writer.u8(update.value);
			break;
		case 3:
			writer.varuint(update.value);
			break;
		case 8:
			writer.boolean(update.value);
			break;
	}
	return writer.done();
}
