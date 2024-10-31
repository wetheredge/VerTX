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
	type BooleanSettings,
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
			<input
				id={styles.advancedState}
				type="checkbox"
				hidden
				checked={api[ResponseKind.Config]?.[configKeys.expert]}
			/>

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

			<h2>Display</h2>

			<SettingInput
				key={configKeys.display.brightness}
				label="Brightness"
				type="number"
				min={1}
				max={255}
				handler={handleInteger}
			/>

			<h2>Wi-Fi</h2>

			<SettingInput
				advanced
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

			<h2 id={styles.dangerId}>Danger zone</h2>

			<SettingCheckbox
				key={configKeys.expert}
				label="Enable advanced settings"
				description="These settings are unnecessary for most users and can easily cause problems if configured incorrectly."
			/>
		</>
	);
}

type SettingProps<Key extends keyof Config, E> = {
	advanced?: boolean;
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
		['advanced', 'key', 'label', 'description'],
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
	advanced?: boolean;
	id: string;
	label: string;
	description?: string;
	children: JSX.Element;
}) {
	return (
		<div
			class={styles.setting}
			classList={{ [styles.advancedSetting]: props.advanced }}
		>
			<label for={props.id}>{props.label}</label>
			<Show when={props.description}>
				<span id={descriptionId(props.id)}>{props.description}</span>
			</Show>
			{props.children}
		</div>
	);
}

function SettingCheckbox<Key extends BooleanSettings>(
	props: Omit<SettingProps<Key, never>, 'handler'>,
) {
	const id = createUniqueId();

	let input!: HTMLInputElement;
	const checked = () => api[ResponseKind.Config]?.[props.key] ?? false;

	return (
		<div
			class={styles.settingCheckbox}
			classList={{ [styles.advancedSetting]: props.advanced }}
		>
			<input
				id={id}
				type="checkbox"
				aria-describedby={props.description && descriptionId(id)}
				checked={checked()}
				onChange={async ({ target }) => {
					const result = await updateConfig({
						key: props.key,
						value: target.checked,
					});
					input.checked = checked();
					handleUpdateResult(props.key, result);
				}}
				ref={input}
			/>
			<label for={id}>{props.label}</label>
			<Show when={props.description}>
				<span id={descriptionId(id)}>{props.description}</span>
			</Show>
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
