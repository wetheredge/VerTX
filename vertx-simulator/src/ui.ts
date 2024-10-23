export class Ui {
	#status: HTMLDivElement;
	#power: HTMLButtonElement;
	#configurator: HTMLButtonElement;

	constructor(callbacks: { power: () => void; configurator: () => void }) {
		this.#status = document.querySelector('#status-led')!;
		this.#power = document.querySelector('#power')!;
		this.#configurator = document.querySelector('#config')!;

		this.#power.disabled = true;
		this.#configurator.disabled = true;

		this.#power.addEventListener('click', callbacks.power);
		this.#configurator.addEventListener('click', callbacks.configurator);
	}

	ready() {
		this.#power.disabled = false;
	}

	start() {
		this.#configurator.disabled = false;
	}

	stop() {
		this.#configurator.disabled = true;
	}

	setStatusColor(color: string) {
		this.#status.style.color = color;
	}
}
