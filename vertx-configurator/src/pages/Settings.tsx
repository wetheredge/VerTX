import { type JSX, Show, createUniqueId, splitProps } from 'solid-js';
import {
	type ConfigUpdateResult,
	ConfigUpdateResultKind,
	ResponseKind,
	api,
	configUpdateResultToString,
	updateConfig,
} from '../api';
import {
	type Config,
	type EnumSettings,
	type IntegerSettings,
	type StringSettings,
	configKeys,
} from '../config';
import * as styles from './Settings.css';

type Handler<Key extends keyof Config, E> = (
	key: Key,
	reset: () => void,
) => JSX.ChangeEventHandlerUnion<E, Event>;

const handleString: Handler<
	StringSettings,
	HTMLInputElement | HTMLSelectElement
> =
	(key, reset) =>
	async ({ target }) => {
		const result = await updateConfig({ key, value: target.value });
		reset();
		handleUpdateResult(key, result);
	};

const handleInteger: Handler<
	IntegerSettings | EnumSettings,
	HTMLInputElement | HTMLSelectElement
> =
	(key, reset) =>
	async ({ target }) => {
		let value = Number.parseInt(target.value);
		if ('max' in target) {
			value = Math.min(value, Number.parseInt(target.max));
		}
		if ('min' in target) {
			value = Math.max(value, Number.parseInt(target.min));
		}
		target.value = value.toString();
		const result = await updateConfig({ key, value });
		reset();
		handleUpdateResult(key, result);
	};

export default function Settings() {
	return (
		<>
			<h1>Settings</h1>

			<SettingInput
				key={configKeys.name}
				label="Device name"
				description="Used in the Wi-Fi SSID to differentiate this VerTX handset from any others in range."
				type="text"
				maxlength={20}
				handler={handleString}
			/>

			<SettingInput
				key={configKeys.leds.brightness}
				label="Status LED brightness"
				type="number"
				min={10}
				max={255}
				handler={handleInteger}
			/>

			<h2>Backup</h2>
			<p>TODO: backup & restore buttons</p>

			<h2>Wi-Fi</h2>

			<SettingInput
				key={configKeys.network.password}
				label="Password"
				type="password"
				maxlength={64}
				handler={handleString}
			/>

			<h3>Home</h3>

			<SettingInput
				key={configKeys.network.home.ssid}
				label="SSID"
				type="text"
				maxlength={32}
				handler={handleString}
			/>
			<SettingInput
				key={configKeys.network.home.password}
				label="Password"
				type="password"
				maxlength={64}
				handler={handleString}
			/>
		</>
	);
}

type SettingProps<Key extends keyof Config, E> = {
	key: Key;
	label: string;
	description?: string;
	handler: Handler<Key, E>;
};

const descriptionId = (id: string) => `${id}d`;

type InputProps<Key> = Key extends StringSettings
	? { type: 'text' | 'password'; minlength?: number; maxlength?: number }
	: Key extends IntegerSettings
		? { type: 'number'; min?: number; max?: number; step?: number }
		: never;
function SettingInput<Key extends StringSettings | IntegerSettings>(
	props: SettingProps<Key, HTMLInputElement> & InputProps<Key>,
) {
	const id = createUniqueId();
	const [baseProps, , inputProps] = splitProps(
		props,
		['key', 'label', 'description'],
		['handler'],
	);

	let input!: HTMLInputElement;
	const value = () => api[ResponseKind.Config]?.[props.key]?.toString() ?? '';
	const resetInput = () => {
		input.value = value();
	};

	return (
		<SettingBase id={id} {...baseProps}>
			<input
				{...inputProps}
				id={id}
				aria-describedby={props.description && descriptionId(id)}
				onChange={props.handler(props.key, resetInput)}
				value={value()}
				ref={input}
			/>
		</SettingBase>
	);
}

function SettingBase(props: {
	id: string;
	label: string;
	description?: string;
	children: JSX.Element;
}) {
	return (
		<div class={styles.setting}>
			<label for={props.id}>{props.label}</label>
			<Show when={props.description}>
				<span id={descriptionId(props.id)}>{props.description}</span>
			</Show>
			{props.children}
		</div>
	);
}

function handleUpdateResult(key: keyof Config, result: ConfigUpdateResult) {
	if (result.result !== ConfigUpdateResultKind.Ok) {
		console.error(
			`Failed to save '${key}':`,
			configUpdateResultToString(result),
		);
	}
}
