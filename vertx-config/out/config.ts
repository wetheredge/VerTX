import type { Reader, Writer } from 'postcard';

export const configKeys = {
	name: 0,
	leds: {
		brightness: 1,
	},
	network: {
		hostname: 2,
		password: 3,
		home: {
			ssid: 4,
			password: 5,
		},
	},
} as const;

export type Config = {
	0: string;
	1: number;
	2: string;
	3: string;
	4: string;
	5: string;
};

export function parseConfig(reader: Reader): Config {
	// Ignore u32 version
	for (let i = 0; i < 4; i++) {
		reader.u8();
	}
	return [
		reader.string(),
		reader.u8(),
		reader.string(),
		reader.string(),
		reader.string(),
		reader.string(),
	];
}

export type StringSettings = 0 | 2 | 3 | 4 | 5;
export type IntegerSettings = 1;
export type EnumSettings = never;
export type BooleanSettings = never;

export type Update =
	| { key: StringSettings; value: string }
	| { key: IntegerSettings | EnumSettings; value: number }
	| { key: BooleanSettings; value: boolean };

export function encodeUpdate(writer: Writer, update: Update): ArrayBuffer {
	writer.varuint(update.key);
	switch (update.key) {
		case 0:
		case 2:
		case 3:
		case 4:
		case 5:
			writer.string(update.value);
			break;
		case 1:
			writer.u8(update.value);
			break;
	}
	return writer.done();
}
