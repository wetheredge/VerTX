import { createSignal } from 'solid-js';

import createApi from '~/lib/api';
import { unreachable } from '~/lib/utils';
import { Button } from './ui/button';
import { Toaster } from './ui/toast';
import { RequestKind, ResponseKind } from '~/lib/api';

const DEV_API_PORT = 8080;
const API_HOST = import.meta.env.PROD
	? location.host
	: import.meta.env.CODESPACE_NAME
		? `${import.meta.env.CODESPACE_NAME}-${DEV_API_PORT}.app.github.dev`
		: `localhost:${DEV_API_PORT}`;

export default function App() {
	const [version, setVersion] = createSignal<string>();

	const api = createApi(API_HOST, ({ kind, payload }) => {
		switch (kind) {
			case ResponseKind.ProtocolVersion:
				console.log(ResponseKind[kind], payload);
				break;
			case ResponseKind.BuildInfo:
				setVersion(
					`v${payload.major}.${payload.minor}.${payload.patch}` +
						(payload.suffix ? `-${payload.suffix}` : ''),
				);
				break;
			case ResponseKind.Status:
				console.log(ResponseKind[kind], payload);
				break;

			default:
				unreachable(kind);
		}
	});

	api.request({ kind: RequestKind.BuildInfo });

	return (
		<>
			<h1>VHS {version()}</h1>

			<Button onClick={() => api.request({ kind: RequestKind.Reboot })}>
				Reboot
			</Button>
			<Button onClick={() => api.request({ kind: RequestKind.PowerOff })}>
				PowerOff
			</Button>

			<Toaster />
		</>
	);
}
