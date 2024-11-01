import { Button } from './simulator';

export class Ui {
	#status: HTMLDivElement;
	#power: HTMLButtonElement;
	#display: HTMLCanvasElement;

	constructor(callbacks: { power(): void; button(id: Button): void }) {
		const app: HTMLDivElement = document.querySelector('#app')!;

		this.#status = app.querySelector('#status-led')!;
		this.#power = app.querySelector('#power')!;
		this.#display = app.querySelector('#display')!;

		this.#power.disabled = true;

		this.#power.addEventListener('click', callbacks.power);

		document.addEventListener('keydown', (event) => {
			// `key` prefix to silence biome's lint/style/useNamingConvention lint
			const keyToButton: Record<string, Button | null> = {
				kArrowUp: Button.Up,
				kArrowDown: Button.Down,
				kArrowRight: Button.Forward,
				kArrowLeft: Button.Back,
			};
			const button = keyToButton[`k${event.key}`];
			if (button != null) {
				event.preventDefault();
				callbacks.button(button);
			}
		});
	}

	get display(): HTMLCanvasElement {
		return this.#display;
	}

	ready() {
		this.#power.disabled = false;
	}

	setStatusColor(color: string) {
		this.#status.style.color = color;
	}
}
