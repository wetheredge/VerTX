import { createSignal, onMount } from 'solid-js';
import { StatusBar } from './StatusBar';
import createApi, {
	type ResponsePayload,
	RequestKind,
	ResponseKind,
} from './api';
import { unreachable } from './utils';

const DEV_API_PORT = 8080;
const API_HOST =
	import.meta.env.MODE === 'production'
		? location.host
		: import.meta.env.CODESPACE_NAME
		  ? `${import.meta.env.CODESPACE_NAME}-${DEV_API_PORT}.app.github.dev`
		  : `localhost:${DEV_API_PORT}`;

export function App() {
	const [build, setBuild] =
		createSignal<ResponsePayload<ResponseKind.BuildInfo>>();
	const [status, setStatus] =
		createSignal<ResponsePayload<ResponseKind.Status>>();

	const api = createApi(API_HOST, ({ kind, payload }) => {
		switch (kind) {
			case ResponseKind.ProtocolVersion:
				console.debug(ResponseKind[kind], payload);
				break;
			case ResponseKind.BuildInfo:
				setBuild(payload);
				break;
			case ResponseKind.Status:
				setStatus(payload);
				break;
			case ResponseKind.Inputs:
				console.debug('inputs', payload);
				break;
			case ResponseKind.Outputs:
				console.debug('outputs', payload);
				break;

			default:
				unreachable(kind);
		}
	});

	api.request({ kind: RequestKind.BuildInfo });

	onMount(() => {
		// @ts-ignore
		globalThis.streamInputs = (payload: boolean) =>
			api.request({ kind: RequestKind.StreamInputs, payload });
		// @ts-ignore
		globalThis.streamOutputs = (payload: boolean) =>
			api.request({ kind: RequestKind.StreamOutputs, payload });
	});

	return (
		<>
			<StatusBar
				build={build()}
				status={status()}
				apiStatus={api.status()}
			/>
		</>
	);
}
