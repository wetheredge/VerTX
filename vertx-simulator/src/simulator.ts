import {
	apiTx,
	buttonPressed,
	initSync,
	memoryName,
} from '../../target/simulator/vertx.js';
import wasmUrl from '../../target/simulator/vertx_bg.wasm?url';

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
		this.loadConfig = this.loadConfig.bind(this);
		this.saveConfig = this.saveConfig.bind(this);
		this.setStatusLed = this.setStatusLed.bind(this);
		this.powerOff = this.powerOff.bind(this);
		this.flushDisplay = this.flushDisplay.bind(this);

		window.addEventListener('message', (event) => {
			if (
				event.origin !== location.origin ||
				!(event.data instanceof ArrayBuffer)
			) {
				return;
			}

			this.#configurator = event.source;

			apiTx(new Uint8Array(event.data));
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

	buttonPressed(button: Button) {
		buttonPressed(button);
	}

	// Used from wasm:

	private openConfigurator() {
		this.#callbacks.openConfigurator();
	}

	private apiRx(raw: Uint8Array) {
		const start = raw.byteOffset;
		const end = start + raw.byteLength;
		this.#configurator?.postMessage(raw.buffer.slice(start, end));
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
