import './style.css';
import { type Callbacks, Simulator, getModule } from './simulator';
import { Ui } from './ui';

let simulator: Simulator | null;
const ui = new Ui({
	power() {
		if (simulator == null) {
			start();
		}
	},
	configurator() {
		if (simulator != null) {
			simulator.modeButtonPressed();
		}
	},
});

const module = await getModule();
ui.ready();

const callbacks: Callbacks = {
	setStatusLed(color) {
		ui.setStatusColor(color);
	},
	shutDown() {
		simulator = null;
		ui.stop();
	},
	reboot(bootMode) {
		start(bootMode);
	},
};

function start(bootMode?: number) {
	simulator = new Simulator(module, callbacks, bootMode);
	ui.start();
}
