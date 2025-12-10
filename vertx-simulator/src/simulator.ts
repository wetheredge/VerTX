import {
	apiTx,
	buttonPressed,
	initSync,
	memoryName,
} from '../../out/firmware/simulator/vertx.js';
import wasmUrl from '../../out/firmware/simulator/vertx_bg.wasm?url';
import { type ConfiguratorResponse, isRequest } from './common.ts';

const globalName = 'Vertx';
const getFileKey = (path: string) => `file:${path}`;

declare const SimulatorModuleTag: unique symbol;
export type SimulatorModule = {
	[SimulatorModuleTag]: true;
} & WebAssembly.Module;

export function getModule(): Promise<SimulatorModule> {
	return WebAssembly.compileStreaming(
		fetch(wasmUrl),
	) as Promise<SimulatorModule>;
}

export const enum Button {
	Up,
	Down,
	Forward,
	Back,
}

export type Callbacks = {
	setStatusLed: (color: string) => void;
	shutDown: () => void;
	reboot: () => void;
	openConfigurator: () => void;
};

export class Simulator {
	readonly #callbacks: Callbacks;
	#configurator: MessageEventSource | null = null;

	readonly #memory: WebAssembly.Memory;
	readonly #display: CanvasRenderingContext2D;

	constructor(
		module: SimulatorModule,
		display: HTMLCanvasElement,
		callbacks: Callbacks,
	) {
		if (globalName in globalThis) {
			throw new Error('Simulator is already running');
		}

		this.openConfigurator = this.openConfigurator.bind(this);
		this.apiRx = this.apiRx.bind(this);
		this.fsList = this.fsList.bind(this);
		this.fsRead = this.fsRead.bind(this);
		this.fsWrite = this.fsWrite.bind(this);
		this.fsDelete = this.fsDelete.bind(this);
		this.setStatusLed = this.setStatusLed.bind(this);
		this.powerOff = this.powerOff.bind(this);
		this.flushDisplay = this.flushDisplay.bind(this);

		window.addEventListener('message', (event) => {
			if (
				!isRequest(event.data) ||
				(import.meta.env.PROD && event.origin !== location.origin)
			) {
				return;
			}

			this.#configurator = event.source;

			const request = event.data;
			const body = request.body ? new Uint8Array(request.body) : null;
			apiTx(request.id, request.route, request.method, body);
		});

		// @ts-expect-error: globalName isn't declared as a property of globalThis
		globalThis[globalName] = this;

		const memory = initSync({ module })[memoryName];
		this.#memory = memory;
		this.#callbacks = callbacks;

		const displayContext = display.getContext('2d');
		if (displayContext == null) {
			throw new Error('Failed to get display context');
		}
		displayContext.imageSmoothingEnabled = false;
		this.#display = displayContext;
	}

	buttonPressed(button: Button) {
		buttonPressed(button);
	}

	// Used from wasm:

	private openConfigurator() {
		this.#callbacks.openConfigurator();
	}

	private apiRx(
		id: number,
		status: number,
		json: boolean,
		body: Uint8Array<ArrayBuffer>,
	) {
		const headers = {
			'Content-Type': json
				? 'application/json'
				: 'application/octet-stream',
		};

		const bodyStart = body.byteOffset;
		const response: ConfiguratorResponse = {
			vertx: 'response',
			id,
			status,
			headers,
			body: body.buffer.slice(bodyStart, bodyStart + body.byteLength),
		};

		const options: WindowPostMessageOptions = {
			transfer: [
				body.buffer.slice(
					body.byteOffset,
					body.byteOffset + body.byteLength,
				),
			],
		};
		if (import.meta.env.DEV) {
			options.targetOrigin = '*';
		}
		this.#configurator?.postMessage(response, options);
	}

	private fsList(path: string): Array<string> {
		if (!path.endsWith('/')) {
			throw new Error('Missing trailing slash');
		}

		const dir = getFileKey(path);
		return Object.keys(localStorage)
			.filter((key) => key.startsWith(dir))
			.map((key) => key.replace(dir, ''));
	}

	private fsRead(path: string): Uint8Array | null {
		const base64 = localStorage.getItem(getFileKey(path));
		if (base64 == null) {
			return null;
		}

		return Uint8Array.fromBase64(base64);
	}

	private fsWrite(path: string, data: Uint8Array) {
		const base64 = data.toBase64();
		localStorage.setItem(getFileKey(path), base64);
	}

	private fsDelete(path: string) {
		localStorage.removeItem(getFileKey(path));
	}

	private setStatusLed(r: number, g: number, b: number) {
		this.#callbacks.setStatusLed(`rgb(${r} ${g} ${b})`);
	}

	private powerOff(restart: boolean) {
		if (restart) {
			this.#callbacks.reboot();
		} else {
			this.#callbacks.shutDown();
		}
	}

	private flushDisplay(ptr: number) {
		const width = 128;
		const height = 64;
		const bitsPerByte = 8;

		// Pixels are stored as 1 bit per pixel
		const sourceLength = (width / bitsPerByte) * height;
		const source = new Uint8Array(this.#memory.buffer, ptr, sourceLength);
		const output = this.#display.createImageData(width, height);

		for (let i = 0; i < width * height; i++) {
			// Pixel `i` is within this byte …
			const pixelByte = Math.floor(i / bitsPerByte);
			// … at this bit offset
			const subByteOffset = i % bitsPerByte;

			// biome-ignore lint/style/noNonNullAssertion: for loop keeps this in bounds
			const color = source[pixelByte]! & (1 << subByteOffset);
			setBinaryColor(output.data, color > 0, i);
		}

		this.#display.putImageData(output, 0, 0);
	}
}

function setBinaryColor(
	data: ImageDataArray,
	isWhite: boolean,
	pixelOffset: number,
) {
	const bytesPerColor = 4;
	const byteOffset = pixelOffset * bytesPerColor;

	// 8 bits per pixel channel range:
	const min = 0;
	const max = 255;

	const raw = isWhite ? max : min;
	// red, green, blue, alpha
	data.set([raw, raw, raw, max], byteOffset);
}
