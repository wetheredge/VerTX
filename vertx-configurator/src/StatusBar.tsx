import { MenuButton } from './MenuButton';
import * as styles from './StatusBar.css';
import { ApiStatus, ResponseKind, type ResponsePayload, api } from './api';
import { formatVersion } from './utils';

const connectionMessage = {
	[ApiStatus.Connected]: 'Connected',
	[ApiStatus.Connecting]: 'Connecting',
	[ApiStatus.NotConnected]: 'Not connected',
};

export function StatusBar() {
	// Hyphen-minus characters in these placeholders will get turned into minus signs
	// by Inter's tnum, reducing width changes when replaced with the real values
	const cpuUsage = (status?: ResponsePayload<ResponseKind.Status>) =>
		status ? (100 * (1 - status.idleTime)).toFixed(1) : '--.-';
	const voltage = () =>
		api[ResponseKind.Status]?.batteryVoltage.toFixed(2) ?? '-.--';

	return (
		<div class={styles.root}>
			<span>
				<MenuButton />
				<span class={styles.vertxWithVersion}>
					VerTX {formatVersion(api[ResponseKind.BuildInfo])}
				</span>
				<span class={styles.vertxWithoutVersion}>VerTX</span>
			</span>
			<span class={styles.apiStatus[api.status]}>
				{connectionMessage[api.status]}
			</span>
			<span>
				<span>{cpuUsage(api[ResponseKind.Status])}%</span>
				<span>{voltage()}V</span>
			</span>
		</div>
	);
}
