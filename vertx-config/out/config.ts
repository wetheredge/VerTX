import type { Reader, Writer } from 'postcard';

export const configKeys = {
	name: 0,
	leds: {
		brightness: 1,
	},
	display: {
		brightness: 2,
	},
	network: {
		hostname: 3,
		password: 4,
		home: {
			ssid: 5,
			password: 6,
		},
	},
	expert: 7,
} as const;

export type Config = {
	0: string;
	1: number;
	2: number;
	3: string;
	4: string;
	5: string;
	6: string;
	7: boolean;
};

export function parseConfig(reader: Reader): Config {
	// Ignore u32 version
	for (let i = 0; i < 4; i++) {
		reader.u8();
	}
	return [
		reader.string(),
		reader.u8(),
		reader.u8(),
		reader.string(),
		reader.string(),
		reader.string(),
		reader.string(),
		reader.boolean(),
	];
}

export type StringSettings = 0 | 3 | 4 | 5 | 6;
export type IntegerSettings = 1 | 2;
export type EnumSettings = never;
export type BooleanSettings = 7;

export type Update =
	| { key: StringSettings; value: string }
	| { key: IntegerSettings | EnumSettings; value: number }
	| { key: BooleanSettings; value: boolean };

export function encodeUpdate(writer: Writer, update: Update): ArrayBuffer {
	writer.varuint(update.key);
	switch (update.key) {
		case 0:
		case 3:
		case 4:
		case 5:
		case 6:
			writer.string(update.value);
			break;
		case 1:
		case 2:
			writer.u8(update.value);
			break;
		case 7:
			writer.boolean(update.value);
			break;
	}
	return writer.done();
}
