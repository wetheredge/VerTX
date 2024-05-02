import { FlipHorizontal2 } from 'lucide-solid';
import { For, createUniqueId } from 'solid-js';
import { ButtonIcon } from '../ButtonIcon';
import * as styles from './Hardware.css';

const analogInputs: Array<Parameters<typeof AnalogInput>[0]> = (
	[
		['Yaw', true],
		['Throttle'],
		['Roll', true],
		['Pitch', true],
		['A0'],
		['A1'],
	] as const
).map(([name, center]) => ({ name, center, value: Math.random() }));

export default function Hardware() {
	return (
		<>
			<h1>Hardware</h1>

			<h2>Analog</h2>

			<div class={styles.inputs}>
				<For each={analogInputs}>
					{(props) => <AnalogInput {...props} />}
				</For>
			</div>

			<button class={styles.calibrate} type="button">
				Start calibration
			</button>

			<h2>Switches</h2>

			<div class={styles.inputs}>
				<SwitchInput name="S0" value={-1} />
				<SwitchInput name="S1" value={1} />
				<SwitchInput name="S2" value={-1} />
				<SwitchInput name="S3" value={0} />
				<SwitchInput name="S4" value={1} />
			</div>
		</>
	);
}

function AnalogInput(props: { name: string; value: number; center?: boolean }) {
	return (
		<RawInput
			name={props.name}
			label={`Analog input ${props.name}`}
			value={props.value}
			center={props.center}
		/>
	);
}

function SwitchInput(props: { name: string; value: -1 | 0 | 1 }) {
	return (
		<RawInput
			center
			name={props.name}
			label={`Switch input ${props.name}`}
			value={(props.value + 1) / 2}
		/>
	);
}

function RawInput(props: {
	name: string;
	label: string;
	value: number;
	center?: boolean;
}) {
	const id = createUniqueId();
	return (
		<>
			<span aria-hidden="true">{props.name}</span>
			<span
				id={id}
				class={styles.analogChannel}
				classList={{ center: props.center }}
				style={`--value: ${props.value * 100}%`}
				role="meter"
				aria-label={props.label}
				aria-valuenow={(
					(props.center ? 2 * props.value - 1 : props.value) * 100
				).toFixed(1)}
				aria-valuemin={props.center ? -100 : 0}
				aria-valuemax={100}
			/>
			<button
				class={styles.iconButton}
				type="button"
				title="Reverse"
				aria-controls={id}
			>
				<ButtonIcon light icon={FlipHorizontal2} />
			</button>
		</>
	);
}
