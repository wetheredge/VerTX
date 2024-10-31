import { MenuButton } from './MenuButton';
import * as styles from './StatusBar.css';
import { ApiStatus, ResponseKind, api } from './api';
import { version } from './utils';

const connectionMessage = {
	[ApiStatus.Connected]: 'Connected',
	[ApiStatus.Connecting]: 'Connecting',
	[ApiStatus.NotConnected]: 'Not connected',
};

export function StatusBar() {
	const voltage = () => api[ResponseKind.Vbat]?.toFixed(2) ?? '-.--';

	return (
		<div class={styles.root}>
			<span>
				<MenuButton />
				<span class={styles.vertxWithVersion}>VerTX {version()}</span>
				<span class={styles.vertxWithoutVersion}>VerTX</span>
			</span>
			<span class={styles.apiStatus[api.status]}>
				{connectionMessage[api.status]}
			</span>
			<span>{voltage()}V</span>
		</div>
	);
}
