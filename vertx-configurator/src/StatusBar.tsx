import * as styles from './StatusBar.css';
import { ApiStatus, type ResponseKind, type ResponsePayload } from './api';

const connectionMessage = {
	[ApiStatus.Connected]: 'Connected',
	[ApiStatus.Connecting]: 'Connecting',
	[ApiStatus.LostConnection]: 'Lost connection',
};

export function StatusBar(props: {
	build?: ResponsePayload<ResponseKind.BuildInfo>;
	status?: ResponsePayload<ResponseKind.Status>;
	apiStatus: ApiStatus;
}) {
	const cpuUsage = () =>
		props.status ? (100 * (1 - props.status.idleTime)).toFixed(1) : '--';
	const voltage = () =>
		props.status ? props.status.batteryVoltage.toFixed(2) : '-.--';

	return (
		<div class={styles.root}>
			<span>VerTX {props.build && version(props.build)}</span>
			<span class={styles.apiStatus[props.apiStatus]}>
				{connectionMessage[props.apiStatus]}
			</span>
			<span class={styles.right}>
				<span>{cpuUsage()}%</span>
				<span>{voltage()}V</span>
			</span>
		</div>
	);
}

function version(build: ResponsePayload<ResponseKind.BuildInfo>): string {
	const suffix = build.suffix ? `-${build.suffix}` : '';
	return `v${build.major}.${build.minor}.${build.patch}${suffix}`;
}
