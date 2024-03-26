import { MenuButton } from './MenuButton';
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
	// Hyphen-minus characters in these placeholders will get turned into minus signs
	// by Inter's tnum, reducing width changes when replaced with the real values
	const cpuUsage = () =>
		props.status ? (100 * (1 - props.status.idleTime)).toFixed(1) : '--.-';
	const voltage = () =>
		props.status ? props.status.batteryVoltage.toFixed(2) : '-.--';

	return (
		<div class={styles.root}>
			<span>
				<MenuButton />
				<span class={styles.vertxWithVersion}>
					VerTX {props.build && version(props.build)}
				</span>
				<span class={styles.vertxWithoutVersion}>VerTX</span>
			</span>
			<span class={styles.apiStatus[props.apiStatus]}>
				{connectionMessage[props.apiStatus]}
			</span>
			<span>
				<span>{cpuUsage()}%</span>
				<span>{voltage()}V</span>
			</span>
		</div>
	);
}

function version(build: ResponsePayload<ResponseKind.BuildInfo>): string {
	// Unicode hyphen to avoid Inter's tnum feature making it into a minus sign
	const suffix = build.suffix ? `\u{2011}${build.suffix}` : '';
	return `v${build.major}.${build.minor}.${build.patch}${suffix}`;
}
