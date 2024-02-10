export class DataReader {
	readonly #view: DataView;
	#index = 0;
	#textDecoder = new TextDecoder();

	constructor(buffer: ArrayBuffer) {
		this.#view = new DataView(buffer);
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

	varint(): number {
		const byte = this.u8();
		switch (byte) {
			case 251: {
				const x = this.#view.getUint16(this.#index, true);
				this.#index += 2;
				return x;
			}
			case 252: {
				const x = this.#view.getUint32(this.#index, true);
				this.#index += 4;
				return x;
			}
			case 253:
				throw new Error('cannot parse 64 bit varint');
			case 254:
				throw new Error('cannot parse 128 bit varint');
			case 255:
				throw new Error('invalid varint type byte');
			default:
				return byte;
		}
	}

	string(): string {
		const length = this.varint();
		const s = this.#textDecoder.decode(
			new DataView(this.#view.buffer, this.#index, length),
		);
		this.#index += length;
		return s;
	}
}

export class DataWriter {
	readonly #buffer: ArrayBuffer;
	readonly #view: DataView;
	#index = 0;

	constructor(bytes: number) {
		this.#buffer = new ArrayBuffer(bytes);
		this.#view = new DataView(this.#buffer);
	}

	done(): DataView {
		return new DataView(this.#buffer, 0, this.#index);
	}

	varint(x: number) {
		if (x < 251) {
			this.u8(x);
		} else if (x < 2 ** 16) {
			this.u8(251);
			this.u16(x);
		} else if (x < 2 ** 32) {
			this.u8(252);
			this.u32(x);
		}
	}

	boolean(bool: boolean) {
		this.u8(bool ? 1 : 0);
	}

	u8(x: number) {
		this.#view.setUint8(this.#index++, x);
	}

	u16(x: number) {
		this.#view.setUint16(this.#index, x);
		this.#index += 2;
	}

	u32(x: number) {
		this.#view.setUint32(this.#index, x);
		this.#index += 4;
	}
}
