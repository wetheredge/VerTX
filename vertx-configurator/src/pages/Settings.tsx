import { For, type JSX, Show, splitProps } from 'solid-js';
import * as styles from './Settings.css';

const wifiCountries = [
	{ value: 'ca', label: 'Canada' },
	{ value: 'cn', label: 'China' },
	{ value: 'jp', label: 'Japan' },
	{ value: 'us', label: 'United States' },
];

export default function Settings() {
	return (
		<>
			<h1>Settings</h1>

			<SettingInput
				id="device-name"
				label="Device name"
				description="Used in the Wi-Fi SSID to differentiate this VerTX handset from any others in range."
				type="text"
			/>

			<h2>Backup</h2>
			<p>TODO: backup & restore buttons</p>

			<h2>Display</h2>

			<SettingInput
				id="display-brightness"
				label="Brightness"
				type="number"
				min={1}
				max={255}
			/>
			<SettingSelect
				id="font-size"
				label="Font size"
				options={[
					{ value: 7, label: '7px' },
					{ value: 9, label: '9px' },
				]}
			/>

			<h2>Wi-Fi</h2>

			<SettingSelect
				id="wifi-region"
				label="Country"
				description=""
				options={wifiCountries}
			/>
			<SettingInput
				id="hotspot-password"
				label="Password"
				type="password"
			/>

			<h3>Home</h3>

			<SettingInput id="wifi-ssid" label="SSID" type="text" />
			<SettingInput id="wifi-password" label="Password" type="password" />

			<h2 id={styles.dangerId}>Danger zone</h2>

			<SettingCheckbox
				id="advanced-settings"
				label="Enable advanced settings"
				description="These settings are unnecessary for most users and can easily cause problems if configured incorrectly."
			/>
		</>
	);
}

type SettingProps = { id: string; label: string; description?: string };
type InputProps =
	| { type: 'text' | 'password' }
	| { type: 'number'; min?: number; max?: number; step?: number };

function SettingInput(props: SettingProps & InputProps) {
	const [baseProps, inputProps] = splitProps(props, ['label', 'description']);
	return (
		<SettingBase id={props.id} {...baseProps}>
			<input
				{...inputProps}
				aria-describedby={props.description && `${props.id}-desc`}
			/>
		</SettingBase>
	);
}

function SettingSelect<V extends string | number>(
	props: SettingProps & { options: Array<{ value: V; label: string }> },
) {
	const [baseProps] = splitProps(props, ['id', 'label', 'description']);
	return (
		<SettingBase {...baseProps}>
			<select
				id={props.id}
				aria-describedby={props.description && `${props.id}-desc`}
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

function SettingBase(props: SettingProps & { children: JSX.Element }) {
	return (
		<div class={styles.setting}>
			<label for={props.id}>{props.label}</label>
			<Show when={props.description}>
				<span id={`${props.id}-desc`}>{props.description}</span>
			</Show>
			{props.children}
		</div>
	);
}

function SettingCheckbox(props: SettingProps) {
	const descriptionId = () => `${props.id}-desc`;
	return (
		<div class={styles.settingCheckbox}>
			<input
				id={props.id}
				type="checkbox"
				aria-describedby={props.description && descriptionId()}
			/>
			<label for={props.id}>{props.label}</label>
			<Show when={props.description}>
				<span id={descriptionId()}>{props.description}</span>
			</Show>
		</div>
	);
}
