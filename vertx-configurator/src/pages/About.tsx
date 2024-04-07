import { ResponseKind, type ResponsePayload, api } from '../api';
import { formatVersion } from '../utils';
import * as styles from './About.css';

export default function About() {
	return (
		<>
			<h1>About</h1>
			<BuildInfoTable build={api[ResponseKind.BuildInfo]} />
		</>
	);
}

type BuildInfo = ResponsePayload<ResponseKind.BuildInfo>;

function BuildInfoTable(props: { build?: BuildInfo }) {
	return (
		<table class={styles.buildInfoTable}>
			<tbody>
				<Row name="Target" value={props.build?.target} />
				<Row name="Version" value={formatVersion(props.build)} />
				<Row name="Debug" value={props.build?.debug.toString()} />
				<Row name="Branch" value={props.build?.git.branch} />
				<Row name="Commit" value={formatCommit(props.build)} />
			</tbody>
		</table>
	);
}

function formatCommit(build?: BuildInfo) {
	if (build) {
		const { commit, dirty } = build.git;
		return `${commit}${dirty ? '-dirty' : ''}`;
	}
}

function Row(props: { name: string; value?: string }) {
	return (
		<tr>
			<th>{props.name}</th>
			<td>{props.value}</td>
		</tr>
	);
}
