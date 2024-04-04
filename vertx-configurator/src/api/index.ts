import { makeReconnectingWS } from '@solid-primitives/websocket';
import { onCleanup } from 'solid-js';
import { createStore } from 'solid-js/store';
import {
	type Request,
	type ResponseKind,
	type ResponsePayload,
	encodeRequest,
	parseResponse,
} from './protocol';

export {
	type Request,
	type Response,
	type ResponsePayload,
	ResponseKind,
	RequestKind,
} from './protocol';

export const enum ApiStatus {
	Connecting,
	Connected,
	LostConnection,
}

type State = { status: ApiStatus } & {
	[Kind in ResponseKind]?: ResponsePayload<Kind>;
};

let socket!: WebSocket;
const [state, setState] = createStore<State>({ status: ApiStatus.Connecting });
const setStatus = (status: ApiStatus) => setState('status', status);

export { state as api };
export const request = (request: Request) =>
	socket.send(encodeRequest(request));

export function initApi(host: string) {
	socket = makeReconnectingWS(`ws://${host}/ws`, 'v0', {
		delay: 15_000,
		retries: 5,
	});

	onCleanup(() => socket.close());

	socket.addEventListener('open', () => setStatus(ApiStatus.Connected));
	socket.addEventListener('close', () => setStatus(ApiStatus.LostConnection));

	socket.addEventListener(
		'message',
		async ({ data }: MessageEvent<string | Blob>) => {
			if (data instanceof Blob) {
				const response = parseResponse(await data.arrayBuffer());
				setState(response.kind, response.payload);
			}
		},
	);
}
