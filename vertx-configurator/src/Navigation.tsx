import { A } from '@solidjs/router';
import {
	Gamepad2Icon,
	InfoIcon,
	type LucideIcon,
	MonitorIcon,
	PackageIcon,
	PlaneIcon,
	PlusIcon,
	PowerIcon,
	RotateCcwIcon,
	SettingsIcon,
	SparklesIcon,
	WifiOffIcon,
} from 'lucide-solid';
import { For, onCleanup, onMount } from 'solid-js';
import { ButtonIcon } from './ButtonIcon';
import { closeMenu } from './MenuButton';
import * as styles from './Navigation.css';
import { type Request, RequestKind, request } from './api';

const enum Mixer {
	Basic,
	Plane,
	Simulator,
}
const mixerToIcon: { [_ in Mixer]: LucideIcon } = {
	[Mixer.Basic]: SparklesIcon,
	[Mixer.Plane]: PlaneIcon,
	[Mixer.Simulator]: MonitorIcon,
};

const rawModels: Array<[string, Mixer]> = [
	['LOS quad', Mixer.Basic],
	['FPV quad', Mixer.Basic],
	['3D plane', Mixer.Plane],
	['INAV plane', Mixer.Basic],
	['Sim', Mixer.Simulator],
];
const models = rawModels.map(([label, icon], id) => ({ id, label, icon }));

export function Navigation() {
	const closeOnEscape = ({ key }: { key: string }) => {
		if (key === 'Escape') {
			closeMenu();
		}
	};

	onMount(() => window.addEventListener('keydown', closeOnEscape));
	onCleanup(() => window.removeEventListener('keydown', closeOnEscape));

	return (
		<div class={styles.root}>
			<nav class={styles.nav}>
				<NavLink href="/" label="About" icon={InfoIcon} />
				<NavLink
					href="/settings"
					label="Settings"
					icon={SettingsIcon}
				/>
				<NavLink href="/updates" label="Updates" icon={PackageIcon} />
				<NavLink
					href="/hardware"
					label="Hardware"
					icon={Gamepad2Icon}
				/>
				<div class={styles.modelsHeader}>
					<span>Models</span>
					<button
						type="button"
						class={styles.newModel}
						onClick={() => console.error('TODO: create new model')}
						aria-label="New model"
					>
						<ButtonIcon icon={PlusIcon} />
					</button>
				</div>
				<For each={models}>
					{({ id, label, icon }) => (
						<NavLink
							href={`/model/${id}`}
							label={label}
							icon={mixerToIcon[icon]}
						/>
					)}
				</For>
				<div class={styles.powerButtonsRow}>
					<PowerButton
						label="Exit configurator"
						icon={WifiOffIcon}
						request={{ kind: RequestKind.ExitConfigurator }}
					/>
					<PowerButton
						label="Power off"
						icon={PowerIcon}
						request={{ kind: RequestKind.PowerOff }}
					/>
					<PowerButton
						label="Reboot"
						icon={RotateCcwIcon}
						request={{ kind: RequestKind.Reboot }}
					/>
				</div>
			</nav>
		</div>
	);
}

function NavLink(props: {
	label: string;
	icon: LucideIcon;
	href: string;
}) {
	return (
		<A href={props.href} end onClick={closeMenu}>
			<props.icon
				class={styles.navIcon}
				size="1em"
				strokeWidth="2"
				aria-hidden="true"
			/>
			<span>{props.label}</span>
		</A>
	);
}

function PowerButton(props: {
	label: string;
	icon: LucideIcon;
	request: Request;
}) {
	// TODO: modal confirmation dialog
	return (
		<button
			type="button"
			class={styles.powerButton}
			aria-label={props.label}
			onClick={() => request(props.request)}
		>
			<ButtonIcon light icon={props.icon} />
		</button>
	);
}
