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
	button(id) {
		simulator?.buttonPressed?.(id);
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
	},
	reboot(bootMode) {
		start(bootMode);
	},
	openConfigurator() {
		window.open('/configurator/', 'vertx-configurator');
	},
};

function start(bootMode?: number) {
	simulator = new Simulator(module, ui.display, callbacks, bootMode);
}
