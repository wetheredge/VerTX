import { makeReconnectingWS } from '@solid-primitives/websocket';
import { type Accessor, createSignal, onCleanup } from 'solid-js';
import {
	type Request,
	type Response,
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

export type Api = {
	status: Accessor<ApiStatus>;
	request: (request: Request) => void;
};

export default function createApi(
	host: string,
	onResponse: (resp: Response) => void,
): Api {
	const [status, setStatus] = createSignal(ApiStatus.Connecting);

	const socket = makeReconnectingWS(`ws://${host}/ws`, 'v0', {
		delay: 15_000,
		retries: 5,
	});

	onCleanup(() => {
		socket.close();
	});

	socket.addEventListener('open', () => {
		setStatus(ApiStatus.Connected);
	});

	socket.addEventListener('close', ({ code }) => {
		if (code === 4000) {
			console.error('API in use');
		}

		setStatus(ApiStatus.LostConnection);
	});

	socket.addEventListener(
		'message',
		async ({ data }: MessageEvent<string | Blob>) => {
			if (data instanceof Blob) {
				onResponse(parseResponse(await data.arrayBuffer()));
			}
		},
	);

	return {
		status,
		request(request) {
			socket.send(encodeRequest(request));
		},
	};
}
