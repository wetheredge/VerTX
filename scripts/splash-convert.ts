#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { argv, stdout } from 'node:process';
import { type DecodedPng, decode } from 'fast-png';
import { panic } from '#utils/cli';

const WIDTH = 128;
const HEIGHT = 64;

// Largest power of 2 that can safely be represented in a JS number
const SAFE_BITS = 16;

const path = argv[2];
if (!path) {
	panic('Missing input path');
}

const rawImage = readFileSync(path);
const image = decode(rawImage, { checkCrc: true });
if (image.width !== WIDTH || image.height !== HEIGHT) {
	panic('Incorrect dimensions');
}

const output = ['static SPLASH: [u128;64] = [\n'];
for (let i = 0; i < WIDTH * HEIGHT; ) {
	if (i % WIDTH === 0) {
		output.push('    0x');
	}

	const bits: Array<number> = [];
	for (; bits.length < SAFE_BITS; i++) {
		bits.push(getBinaryPixel(image, i));
	}
	output.push(bitsToHex(bits));

	if (i % WIDTH === 0) {
		output.push(',\n');
	}
}
output.push('];\n');
writeFileSync(stdout.fd, output.join(''));

function getBinaryPixel(image: DecodedPng, pixel: number): number {
	// Maximum channel value for source image bit depth
	const max = 2 ** image.depth - 1;

	const start = pixel * image.channels;
	const end = (pixel + 1) * image.channels;
	const monochrome = average(...image.data.slice(start, end));
	return monochrome / max;
}

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

function average(...values: Array<number>): number {
	if (values.length === 0) {
		throw new RangeError('Cannot average zero inputs');
	}

	return values.reduce((acc, curr) => acc + curr) / values.length;
}
