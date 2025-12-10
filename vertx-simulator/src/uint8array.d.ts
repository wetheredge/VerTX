// Derived from Bun's type declarations under the MIT license

// biome-ignore-all lint/style: copied from Bun

// Source: <https://github.com/oven-sh/bun/blob/c7327d62c24aaa8951b64a9e0e3b70c27f228582/packages/bun-types/globals.d.ts#L1470-L1482>
interface Uint8ArrayConstructor {
	/**
	 * Create a new Uint8Array from a base64 encoded string
	 * @param base64 The base64 encoded string to convert to a Uint8Array
	 * @param options Optional options for decoding the base64 string
	 * @returns A new Uint8Array containing the decoded data
	 */
	fromBase64(
		base64: string,
		options?: {
			alphabet?: 'base64' | 'base64url';
			lastChunkHandling?: 'loose' | 'strict' | 'stop-before-partial';
		},
	): Uint8Array;
}

// Source: <https://github.com/oven-sh/bun/blob/c7327d62c24aaa8951b64a9e0e3b70c27f228582/packages/bun-types/globals.d.ts#L1419-L1423>
interface Uint8Array {
	/**
	 * Convert the Uint8Array to a base64 encoded string
	 * @returns The base64 encoded string representation of the Uint8Array
	 */
	toBase64(options?: {
		alphabet?: 'base64' | 'base64url';
		omitPadding?: boolean;
	}): string;
}
