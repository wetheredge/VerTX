import * as types from './types';

export const version = 1;

export const config = {
	name: types.string(20),
	leds: {
		brightness: types.integer('u8', { min: 10 }),
	},
	display: {
		brightness: types.integer('u8', { min: 1 }),
		fontSize: types.enumeration('FontSize', [
			{ name: '7px', ident: 'Size7px' },
			{ name: '9px', ident: 'Size9px', default: true },
		]),
	},
	network: {
		hostname: types.string(32),
		password: types.string(64),
		home: {
			ssid: types.string(32),
			password: types.string(64),
		},
	},
	expert: types.boolean(),
};
