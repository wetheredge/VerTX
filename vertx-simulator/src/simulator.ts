import {
	apiTx,
	buttonPressed,
	initSync,
	memoryName,
} from '../../out/firmware/simulator/vertx.js';
import wasmUrl from '../../out/firmware/simulator/vertx_bg.wasm?url';
import type { ConfiguratorRequest, ConfiguratorResponse } from './common.js';

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
	#callbacks: Callbacks;
	#configurator: MessageEventSource | null = null;

	#memory: WebAssembly.Memory;
	#display: CanvasRenderingContext2D;

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
		this.storageFileLength = this.storageFileLength.bind(this);
		this.storageRead = this.storageRead.bind(this);
		this.storageWrite = this.storageWrite.bind(this);
		this.storageTruncate = this.storageTruncate.bind(this);
		this.storageDirEntries = this.storageDirEntries.bind(this);
		this.setStatusLed = this.setStatusLed.bind(this);
		this.powerOff = this.powerOff.bind(this);
		this.flushDisplay = this.flushDisplay.bind(this);

		window.addEventListener(
			'message',
			(event: MessageEvent<ConfiguratorRequest>) => {
				// FIXME:
				// if (event.origin !== location.origin) {
				// 	return;
				// }

				this.#configurator = event.source;

				const request = event.data;
				const body = request.body ? new Uint8Array(request.body) : null;
				apiTx(request.id, request.route, request.method, body);
			},
		);

		// @ts-ignore
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

	private storageFileLength(path: string): number {
		return readFile(getFileKey(path)).byteLength;
	}

	private storageRead(
		path: string,
		cursor: number,
		buffer: Uint8Array,
	): number {
		const remaining = readFile(path).slice(cursor);
		const length = Math.min(remaining.byteLength, buffer.byteLength);
		buffer.set(remaining.slice(0, length));
		return length;
	}

	private storageWrite(
		path: string,
		cursor: number,
		data: Uint8Array,
	): number {
		let contents = readFile(path);
		const targetLength = cursor + data.byteLength;
		if (contents.byteLength < targetLength) {
			const resized = new Uint8Array(targetLength);
			resized.set(contents);
			contents = resized;
		}

		contents.set(data, cursor);
		writeFile(path, contents);

		return data.byteLength;
	}

	private storageTruncate(path: string, cursor: number) {
		const contents = readFile(path).slice(0, cursor);
		writeFile(path, contents);
	}

	private storageDirEntries(path: string): Array<string> {
		if (!path.endsWith('/')) {
			throw new Error('Missing trailing slash');
		}

		const dir = getFileKey(path);
		return Object.keys(localStorage)
			.filter((key) => key.startsWith(dir))
			.map((key) => key.replace(dir, ''));
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

		const sourceLength = (width / 8) * height;
		const source = new Uint8Array(this.#memory.buffer, ptr, sourceLength);
		const output = this.#display.createImageData(width, height);

		for (let i = 0; i < width * height; i++) {
			const isSet = source[Math.floor(i / 8)] & (1 << (i % 8));
			const channel = isSet > 0 ? 255 : 0;

			output.data.set([channel, channel, channel, 255], i * 4);
		}

		this.#display.putImageData(output, 0, 0);
	}
}

function readFile(path: string): Uint8Array {
	const base64 = localStorage.getItem(getFileKey(path));
	if (base64 == null) {
		return new Uint8Array();
	}

	return Uint8Array.fromBase64(base64);
}

function writeFile(path: string, contents: Uint8Array) {
	const base64 = contents.toBase64();
	localStorage.setItem(getFileKey(path), base64);
}
