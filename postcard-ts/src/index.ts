export class Reader {
	readonly #view: DataView;
	readonly #textDecoder = new TextDecoder();
	#index = 0;

	constructor(data: DataView) {
		this.#view = data;
	}

	boolean(): boolean {
		const bool = this.u8();
		switch (bool) {
			case 0:
				return false;
			case 1:
				return true;
			default:
				throw new Error(`invalid boolean: ${bool}`);
		}
	}

	u8(): number {
		return this.#view.getUint8(this.#index++);
	}

	f32(): number {
		const float = this.#view.getFloat32(this.#index, true);
		this.#index += 4;
		return float;
	}

	varuint(): number {
		let int = 0;
		let byte: number;
		let i = 0;
		do {
			byte = this.u8();
			int += (byte & 0x7f) << (7 * i++);
		} while (byte & 0x80);
		return int;
	}

	string(): string {
		const length = this.varuint();
		const s = this.#textDecoder.decode(
			new DataView(this.#view.buffer, this.#index, length),
		);
		this.#index += length;
		return s;
	}

	byteArray(): Uint8Array {
		const length = this.varuint();
		const start = this.#index;
		this.#index += length;
		return new Uint8Array(this.#view.buffer.slice(start, start + length));
	}
}

export class Writer {
	readonly #view: DataView<ArrayBuffer>;
	readonly #textEncoder = new TextEncoder();
	#index = 0;

	constructor(bytes: number) {
		const buffer = new ArrayBuffer(bytes);
		this.#view = new DataView(buffer);
	}

	done(): ArrayBuffer {
		return this.#view.buffer.slice(0, this.#index);
	}

	boolean(bool: boolean) {
		this.u8(bool ? 1 : 0);
	}

	u8(x: number) {
		this.#view.setUint8(this.#index++, x);
	}

	varuint(x: number) {
		let remaining = x >>> 0;
		let done: boolean;
		do {
			done = remaining <= 0x7f;
			this.u8(done ? remaining : (remaining & 0x7f) | 0x80);
			remaining >>>= 7;
		} while (!done);
	}

	string(s: string) {
		const encoded = this.#textEncoder.encode(s);
		this.varuint(encoded.length);
		const view = new Uint8Array(this.#view.buffer);
		view.set(encoded, this.#index);
		this.#index += encoded.length;
	}

	byteArray(bytes: Uint8Array) {
		this.varuint(bytes.byteLength);
		const view = new Uint8Array(this.#view.buffer);
		view.set(bytes, this.#index);
		this.#index += bytes.byteLength;
	}
}
