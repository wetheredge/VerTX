import { createSignal } from 'solid-js';
import createApi, {
	ResponsePayload,
	RequestKind,
	ResponseKind,
} from '../lib/api';
import { unreachable } from '../lib/utils';
import { StatusBar } from './StatusBar';

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

			default:
				unreachable(kind);
		}
	});

	api.request({ kind: RequestKind.BuildInfo });

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
