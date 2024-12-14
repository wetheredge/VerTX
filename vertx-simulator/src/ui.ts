import { Button } from './simulator';

export class Ui {
	#status: HTMLDivElement;
	#power: HTMLButtonElement;
	#display: HTMLCanvasElement;
	#touches = new Map<number, [number, number]>();

	constructor(callbacks: { power(): void; button(id: Button): void }) {
		const app: HTMLDivElement = document.querySelector('#app')!;

		this.#status = app.querySelector('#status-led')!;
		this.#power = app.querySelector('#power')!;
		this.#display = app.querySelector('#display')!;

		this.#power.disabled = true;

		this.#power.addEventListener('click', callbacks.power);

		const handleTouchStart = getTouchHandler((touch) => {
			this.#touches.set(touch.identifier, [touch.screenX, touch.screenY]);
		});
		const handleTouchCancel = getTouchHandler((touch) => {
			this.#touches.delete(touch.identifier);
		});
		const handleTouchEnd = getTouchHandler((touch) => {
			const start = this.#touches.get(touch.identifier);
			if (start != null) {
				this.#touches.delete(touch.identifier);

				const [startX, startY] = start;
				const [endX, endY] = [touch.screenX, touch.screenY];

				const changeX = endX - startX;
				const changeY = endY - startY;

				const distance = Math.sqrt(changeX ** 2 + changeY ** 2);
				if (distance < this.#display.clientHeight / 3) {
					return;
				}

				const angle = (Math.atan2(changeY, changeX) / Math.PI) * 180;
				const angleRemainder = Math.abs(angle % 90);
				if (30 < angleRemainder && angleRemainder < 60) {
					// Too diagonal
					return;
				}

				let key = Button.Back;
				switch (Math.round(angle / 90)) {
					case -1:
						key = Button.Up;
						break;
					case 0:
						key = Button.Forward;
						break;
					case 1:
						key = Button.Down;
				}
				callbacks.button(key);
			}
		});
		this.#display.addEventListener('touchstart', handleTouchStart);
		this.#display.addEventListener('touchcancel', handleTouchCancel);
		this.#display.addEventListener('touchend', handleTouchEnd);

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

function getTouchHandler(handle: (touch: Touch) => void) {
	return (event: TouchEvent) => {
		event.preventDefault();
		for (const touch of event.changedTouches) {
			handle(touch);
		}
	};
}
