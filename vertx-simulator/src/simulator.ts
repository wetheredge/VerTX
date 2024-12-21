import { createBlockEncoder, createStreamDecoder } from 'ucobs';
import {
	backpackTx,
	buttonPressed,
	initSync,
	memoryName,
} from '../../target/simulator/vertx.js';
import wasmUrl from '../../target/simulator/vertx_bg.wasm?url';
import {
	INIT,
	type ToBackpack,
	ToBackpackKind,
	type ToMain,
	ToMainKind,
	decode,
	encode,
} from './backpack-ipc';
import { unreachable } from './utils';

const globalName = 'Vertx';
const configStorageKey = 'config';

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
	#backpackInitted = false;
	#callbacks: Callbacks;
	#backpackDecode: (data: Uint8Array) => void;
	#backpackEncode: (data: Uint8Array) => void;
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

		this.backpackRx = this.backpackRx.bind(this);
		this.loadConfig = this.loadConfig.bind(this);
		this.saveConfig = this.saveConfig.bind(this);
		this.setStatusLed = this.setStatusLed.bind(this);
		this.powerOff = this.powerOff.bind(this);
		this.flushDisplay = this.flushDisplay.bind(this);

		this.#backpackDecode = createStreamDecoder((raw) => {
			const message = decode(
				new DataView(raw.buffer, raw.byteOffset, raw.byteLength),
			);
			this.#backpackRxMessage(message);
		});
		this.#backpackEncode = createBlockEncoder((chunk) => {
			backpackTx(chunk);
		});

		window.addEventListener('message', (event) => {
			if (
				event.origin !== location.origin ||
				!(event.data instanceof ArrayBuffer)
			) {
				return;
			}

			this.#configurator = event.source;

			const payload = new Uint8Array(event.data);
			this.send({
				kind: ToMainKind.ApiRequest,
				payload,
			});
		});

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

	send(message: ToMain) {
		if (!this.#backpackInitted) {
			throw new Error('Not yet initialized');
		}

		this.#backpackEncode(encode(message));
	}

	buttonPressed(button: Button) {
		buttonPressed(button);
	}

	stop() {
		// @ts-ignore
		delete globalThis[globalName];
	}

	#backpackRxMessage(message: ToBackpack) {
		switch (message.kind) {
			case ToBackpackKind.StartNetwork:
				this.#callbacks.openConfigurator();
				this.send({ kind: ToMainKind.NetworkUp });
				break;
			case ToBackpackKind.ApiResponse: {
				const { payload } = message;
				const start = payload.byteOffset;
				const end = start + payload.byteLength;
				this.#configurator?.postMessage(
					payload.buffer.slice(start, end),
				);
				break;
			}
			case ToBackpackKind.ShutDown:
			case ToBackpackKind.Reboot:
				this.send({ kind: ToMainKind.PowerAck });
				break;

			default:
				unreachable(message);
		}
	}

	// Used from wasm:

	private backpackRx(raw: Uint8Array) {
		if (this.#backpackInitted) {
			this.#backpackDecode(raw);
		} else if (raw.byteLength === INIT.length) {
			for (let i = 0; i < INIT.length; i++) {
				if (raw[i] !== INIT[i]) {
					console.warn('Invalid backpack init message received');
					return;
				}
			}

			backpackTx(new Uint8Array(INIT));
			this.#backpackInitted = true;
		}
	}

	private loadConfig() {
		return localStorage.getItem(configStorageKey);
	}

	private saveConfig(config: string) {
		localStorage.setItem(configStorageKey, config);
	}

	private setStatusLed(r: number, g: number, b: number) {
		this.#callbacks.setStatusLed(`rgb(${r} ${g} ${b})`);
	}

	private powerOff(restart: boolean) {
		this.stop();
		setTimeout(() => {
			if (restart) {
				this.#callbacks.reboot();
			} else {
				this.#callbacks.shutDown();
			}
		}, 0);
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
