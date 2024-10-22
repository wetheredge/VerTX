import './style.css';
import { type Callbacks, Simulator, getModule } from './simulator';

const statusLed: HTMLDivElement = document.querySelector('#status-led')!;
const powerButton: HTMLButtonElement = document.querySelector('#power')!;
const configuratorButton: HTMLButtonElement =
	document.querySelector('#config')!;

powerButton.disabled = true;
configuratorButton.disabled = true;

const module = await getModule();
powerButton.disabled = false;

const callbacks: Callbacks = {
	setStatusLed(color) {
		statusLed.style.color = color;
	},
	shutDown() {
		simulator = null;
	},
	reboot(bootMode) {
		start(bootMode);
	},
};

let simulator: Simulator | null;

powerButton.addEventListener('click', () => {
	if (simulator == null) {
		start();
	}
});

configuratorButton.addEventListener('click', () => {
	if (simulator != null) {
		simulator.modeButtonPressed();
	}
});

function start(bootMode?: number) {
	simulator = new Simulator(module, callbacks, bootMode);
	configuratorButton.disabled = false;
}
