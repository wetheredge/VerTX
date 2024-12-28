import './style.css';
import { type Callbacks, Simulator, getModule } from './simulator';
import { Ui } from './ui';

const rebootStorageKey = 'VerTX reboot';

let simulator: Simulator | null;
let configurator: WindowProxy | null;

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
		configurator?.close?.();
		location.reload();
	},
	reboot() {
		sessionStorage.setItem(rebootStorageKey, '');
		configurator?.close?.();
		location.reload();
	},
	openConfigurator() {
		configurator = window.open('/configurator/', 'vertx-configurator');
	},
};

function start() {
	simulator = new Simulator(module, ui.display, callbacks);
}

if (sessionStorage.getItem(rebootStorageKey) != null) {
	sessionStorage.removeItem(rebootStorageKey);
	start();
}
