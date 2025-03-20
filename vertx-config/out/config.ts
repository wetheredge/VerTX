import { type Reader, Writer } from 'postcard';

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

export function encodeConfig(config: Config): ArrayBuffer {
	const writer = new Writer(242);
	writer.rawU32(3);
	writer.string(config[0]);
	writer.u8(config[1]);
	writer.string(config[2]);
	writer.string(config[3]);
	writer.string(config[4]);
	writer.string(config[5]);
	return writer.done();
}

export type StringSettings = 0 | 2 | 3 | 4 | 5;
export type IntegerSettings = 1;
export type EnumSettings = never;
export type BooleanSettings = never;
