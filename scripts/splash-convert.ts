#!/usr/bin/env bun

import sharp from 'sharp';
import { panic } from './utils.ts';

const WIDTH = 128;
const HEIGHT = 64;

const path = Bun.argv[2];
if (!path) {
	panic('Missing input path');
}

const image = sharp(path);
const meta = await image.metadata();
if (meta.width !== WIDTH || meta.height !== HEIGHT) {
	panic('Incorrect dimensions');
}

const pixels = await image.raw().toColorspace('b-w').toBuffer();
const output = Bun.stdout.writer();
output.write('const SPLASH: [u128;64] = [\n');
for (let i = 0; i < WIDTH * HEIGHT; ) {
	if (i % WIDTH === 0) {
		output.write('    0x');
	}

	const bits: Array<number> = [];
	for (; bits.length < 16; i++) {
		// biome-ignore lint/style/noNonNullAssertion: known to be non-null after checking the image dimensions above
		bits.push(Math.trunc(pixels.at(i)! / 255));
	}
	output.write(bitsToHex(bits));

	if (i % WIDTH === 0) {
		output.write(',\n');
	}
}
output.write('];\n');

/** Collect (big-endian) bits into a hexadecimal string */
function bitsToHex(bits: Array<number>): string {
	const baseHex = 16;
	const bitsPerHexChar = 4;
	return (
		bits
			// biome-ignore lint/suspicious/noBitwiseOperators: yes collecting bits requires bitwise math
			.reduce((acc, cur) => (acc << 1) | cur)
			.toString(baseHex)
			.padStart(bits.length / bitsPerHexChar, '0')
	);
}
