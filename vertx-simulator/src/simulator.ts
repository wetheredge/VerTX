import { createBlockEncoder, createStreamDecoder } from 'ucobs';
import {
	backpackTx,
	initSync,
	modeButtonPressed,
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

export type Callbacks = {
	setStatusLed: (color: string) => void;
	shutDown: () => void;
	reboot: (bootMode: number) => void;
	openConfigurator: () => void;
};

export class Simulator {
	#backpackInitted = false;
	#bootMode: number;
	#callbacks: Callbacks;
	#backpackDecode: (data: Uint8Array) => void;
	#backpackEncode: (data: Uint8Array) => void;
	#configurator: MessageEventSource | null = null;

	constructor(module: SimulatorModule, callbacks: Callbacks, bootMode = 0) {
		if (globalName in globalThis) {
			throw new Error('Simulator is already running');
		}

		this.backpackRx = this.backpackRx.bind(this);
		this.loadConfig = this.loadConfig.bind(this);
		this.saveConfig = this.saveConfig.bind(this);
		this.setStatusLed = this.setStatusLed.bind(this);
		this.powerOff = this.powerOff.bind(this);

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

		initSync({ module });
		this.#bootMode = bootMode;
		this.#callbacks = callbacks;
	}

	send(message: ToMain) {
		if (!this.#backpackInitted) {
			throw new Error('Not yet initialized');
		}

		this.#backpackEncode(encode(message));
	}

	modeButtonPressed() {
		modeButtonPressed();
	}

	stop() {
		// @ts-ignore
		delete globalThis[globalName];
	}

	#backpackRxMessage(message: ToBackpack) {
		switch (message.kind) {
			case ToBackpackKind.SetBootMode:
				this.#bootMode = message.payload;
				break;
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

			const data = new Uint8Array(INIT.length + 1);
			data.set(INIT);
			data[INIT.length] = this.#bootMode;

			backpackTx(data);
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
				this.#callbacks.reboot(this.#bootMode);
			} else {
				this.#callbacks.shutDown();
			}
		}, 0);
	}
}
