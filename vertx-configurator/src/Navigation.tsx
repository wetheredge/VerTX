import { A } from '@solidjs/router';
import {
	Gamepad2,
	Info,
	Monitor,
	Package,
	Plane,
	Plus,
	Settings,
	Sparkles,
} from 'lucide-solid';
import { For, onCleanup, onMount } from 'solid-js';
import { ButtonIcon } from './ButtonIcon';
import { closeMenu } from './MenuButton';
import * as styles from './Navigation.css';

type Icon = typeof Plus;
const enum Mixer {
	Basic,
	Plane,
	Simulator,
}
const mixerToIcon: { [_ in Mixer]: Icon } = {
	[Mixer.Basic]: Sparkles,
	[Mixer.Plane]: Plane,
	[Mixer.Simulator]: Monitor,
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
				<NavLink href="/" label="About" icon={Info} />
				<NavLink href="/settings" label="Settings" icon={Settings} />
				<NavLink href="/updates" label="Updates" icon={Package} />
				<NavLink href="/hardware" label="Hardware" icon={Gamepad2} />
				<div class={styles.modelsHeader}>
					<span>Models</span>
					<button
						type="button"
						class={styles.newModel}
						onClick={() => console.error('TODO: create new model')}
						aria-label="New model"
					>
						<ButtonIcon icon={Plus} />
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
			</nav>
		</div>
	);
}

function NavLink(props: {
	label: string;
	icon: Icon;
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
