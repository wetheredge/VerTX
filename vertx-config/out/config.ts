import { Reader, Writer } from 'postcard';

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
		reader.varint() as FontSize,
		reader.string(),
		reader.string(),
		reader.string(),
		reader.string(),
		reader.boolean(),
	];
}

export function update<Key extends keyof Config>(
	key: Key,
	value: Config[Key],
): ArrayBuffer {
	const writer = new Writer(74);
	switch (key) {
		case 0:
		case 4:
		case 5:
		case 6:
		case 7:
			writer.string(value);
			break;
		case 1:
		case 2:
			writer.u8(value);
			break;
		case 3:
			writer.varuint(value);
			break;
		case 8:
			writer.boolean(value);
			break;
	}
	return writer.done();
}
