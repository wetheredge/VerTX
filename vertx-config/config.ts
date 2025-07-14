import * as types from './src/types.ts';

export const version = 4;

export const config: types.Config = {
	name: types.string(20, 'VerTX'),
	leds: {
		brightness: types.integer('u8', 10, { min: 10 }),
	},
	network: {
		hostname: types.string(32, 'vertx'),
		ap: {
			password: types.string(64),
		},
		sta: {
			ssid: types.string(32),
			password: types.string(64),
		},
	},
};
