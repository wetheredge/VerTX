import { For, type JSX, Show, createUniqueId, splitProps } from 'solid-js';
import {
	type ConfigUpdate,
	ConfigUpdateKind,
	RequestKind,
	request,
} from '../api';
import * as styles from './Settings.css';

const wifiCountries = [
	{ value: 'ca', label: 'Canada' },
	{ value: 'cn', label: 'China' },
	{ value: 'jp', label: 'Japan' },
	{ value: 'us', label: 'United States' },
];

type Handler<E> = (key: string) => JSX.ChangeEventHandlerUnion<E, Event>;

const handleString: Handler<HTMLInputElement | HTMLSelectElement> =
	(key) =>
	({ target }) =>
		handleChange(key, {
			kind: ConfigUpdateKind.String,
			update: target.value,
		});

const handleInteger: Handler<HTMLInputElement | HTMLSelectElement> =
	(key) =>
	({ target }) => {
		let value = Number.parseInt(target.value);
		if ('max' in target) {
			value = Math.min(value, Number.parseInt(target.max));
		}
		if ('min' in target) {
			value = Math.max(value, Number.parseInt(target.min));
		}
		target.value = value.toString();
		handleChange(key, { kind: ConfigUpdateKind.Unsigned, update: value });
	};

export default function Settings() {
	return (
		<>
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
				min={1}
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
		['key', 'label', 'description'],
		['handler'],
	);
	return (
		<SettingBase id={id} {...baseProps}>
			<input
				{...inputProps}
				id={id}
				aria-describedby={props.description && descriptionId(id)}
				onChange={props.handler(props.key)}
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
	const [baseProps] = splitProps(props, ['key', 'label', 'description']);
	return (
		<SettingBase id={id} {...baseProps}>
			<select
				id={id}
				aria-describedby={props.description && descriptionId(id)}
				onChange={props.handler(props.key)}
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

function SettingCheckbox(props: Omit<SettingProps<never>, 'handler'>) {
	const id = keyToId(props.key);
	return (
		<div class={styles.settingCheckbox}>
			<input
				id={id}
				type="checkbox"
				aria-describedby={props.description && descriptionId(id)}
				onChange={({ target }) =>
					handleChange(props.key, {
						kind: ConfigUpdateKind.Boolean,
						update: target.checked,
					})
				}
			/>
			<label for={id}>{props.label}</label>
			<Show when={props.description}>
				<span id={descriptionId(id)}>{props.description}</span>
			</Show>
		</div>
	);
}

let updates = 0;
function handleChange(key: string, payload: ConfigUpdate): void {
	request({
		kind: RequestKind.ConfigUpdate,
		payload: {
			id: updates++,
			key,
			...payload,
		},
	});
}
