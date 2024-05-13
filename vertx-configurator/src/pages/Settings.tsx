import { For, type JSX, Show, createUniqueId, splitProps } from 'solid-js';
import {
	ConfigUpdateKind,
	type ConfigUpdateResult,
	ConfigUpdateResultKind,
	api,
	configUpdateResultToString,
	updateConfig,
} from '../api';
import * as styles from './Settings.css';

const wifiCountries = [
	{ value: 'ca', label: 'Canada' },
	{ value: 'cn', label: 'China' },
	{ value: 'jp', label: 'Japan' },
	{ value: 'us', label: 'United States' },
];

type Handler<E> = (
	key: string,
	reset: () => void,
) => JSX.ChangeEventHandlerUnion<E, Event>;

const handleString: Handler<HTMLInputElement | HTMLSelectElement> =
	(key, reset) =>
	async ({ target }) => {
		const result = await updateConfig(key, {
			kind: ConfigUpdateKind.String,
			update: target.value,
		});
		reset();
		handleUpdateResult(key, result);
	};

const handleInteger: Handler<HTMLInputElement | HTMLSelectElement> =
	(key, reset) =>
	async ({ target }) => {
		let update = Number.parseInt(target.value);
		if ('max' in target) {
			update = Math.min(update, Number.parseInt(target.max));
		}
		if ('min' in target) {
			update = Math.max(update, Number.parseInt(target.min));
		}
		target.value = update.toString();
		const result = await updateConfig(key, {
			kind: ConfigUpdateKind.Unsigned,
			update,
		});
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
				checked={api.config.expert === true}
			/>

			<h1>Settings</h1>

			<SettingInput
				key="name"
				label="Device name"
				description="Used in the Wi-Fi SSID to differentiate this VerTX handset from any others in range."
				type="text"
				maxlength={20}
				handler={handleString}
			/>

			<SettingInput
				key="leds.brightness"
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
				key="display.brightness"
				label="Brightness"
				type="number"
				min={1}
				max={255}
				handler={handleInteger}
			/>
			<SettingSelect
				key="display.font_size"
				label="Font size"
				options={[
					{ value: 7, label: '7px' },
					{ value: 9, label: '9px' },
				]}
				handler={handleInteger}
			/>

			<h2>Wi-Fi</h2>

			<SettingSelect
				key="wifi.region"
				label="Country"
				description=""
				options={wifiCountries}
				handler={handleString}
			/>
			<SettingInput
				advanced
				key="wifi.password"
				label="Password"
				type="password"
				maxlength={64}
				handler={handleString}
			/>

			<h3>Home</h3>

			<SettingInput
				key="wifi.home_ssid"
				label="SSID"
				type="text"
				maxlength={32}
				handler={handleString}
			/>
			<SettingInput
				key="wifi.home_password"
				label="Password"
				type="password"
				maxlength={64}
				handler={handleString}
			/>

			<h2 id={styles.dangerId}>Danger zone</h2>

			<SettingCheckbox
				key="expert"
				label="Enable advanced settings"
				description="These settings are unnecessary for most users and can easily cause problems if configured incorrectly."
			/>
		</>
	);
}

type SettingProps<E> = {
	advanced?: boolean;
	key: string;
	label: string;
	description?: string;
	handler: Handler<E>;
};

const keyToId = import.meta.env.PROD
	? () => createUniqueId()
	: (key: string) => `setting-${key.replaceAll('.', '-')}`;
const descriptionId = import.meta.env.PROD
	? (id: string) => `${id}d`
	: (id: string) => id.replace('setting', 'description');

type InputProps =
	| { type: 'text' | 'password'; minlength?: number; maxlength?: number }
	| { type: 'number'; min?: number; max?: number; step?: number };
function SettingInput(props: SettingProps<HTMLInputElement> & InputProps) {
	const id = keyToId(props.key);
	const [baseProps, , inputProps] = splitProps(
		props,
		['advanced', 'key', 'label', 'description'],
		['handler'],
	);

	let input!: HTMLInputElement;
	const value = () => api.config[props.key]?.toString();
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

function SettingSelect<V extends string | number>(
	props: SettingProps<HTMLSelectElement> & {
		options: Array<{ value: V; label: string }>;
	},
) {
	const id = keyToId(props.key);
	const [baseProps] = splitProps(props, [
		'advanced',
		'key',
		'label',
		'description',
	]);

	let input!: HTMLSelectElement;
	const value = () => api.config[props.key]?.toString();
	const resetInput = () => {
		input.value = value();
	};

	return (
		<SettingBase id={id} {...baseProps}>
			<select
				id={id}
				aria-describedby={props.description && descriptionId(id)}
				onChange={props.handler(props.key, resetInput)}
				value={value()}
				ref={input}
			>
				<For each={props.options}>
					{({ value, label }) => (
						<option value={value}>{label}</option>
					)}
				</For>
			</select>
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

function SettingCheckbox(props: Omit<SettingProps<never>, 'handler'>) {
	const id = keyToId(props.key);

	let input!: HTMLInputElement;
	const checked = () => api.config[props.key] === true;

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
					const result = await updateConfig(props.key, {
						kind: ConfigUpdateKind.Boolean,
						update: target.checked,
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

function handleUpdateResult(key: string, result: ConfigUpdateResult) {
	if (result.result !== ConfigUpdateResultKind.Ok) {
		console.error(
			`Failed to save '${key}':`,
			configUpdateResultToString(result),
		);
	}
}
