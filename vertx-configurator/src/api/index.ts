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

const DEV_API_PORT = 8080;
const HOST =
	import.meta.env.MODE === 'production'
		? location.host
		: import.meta.env.CODESPACE_NAME
			? `${import.meta.env.CODESPACE_NAME}-${DEV_API_PORT}.app.github.dev`
			: `localhost:${DEV_API_PORT}`;
export const API_BASE = `${HOST}/api`;

export const enum ApiStatus {
	Connecting,
	Connected,
	Reconnecting,
	LostConnection,
}

type State = { status: ApiStatus } & {
	[Kind in ResponseKind]?: ResponsePayload<Kind>;
};

let socket: WebSocket | undefined;
const [state, setState] = createStore<State>({ status: ApiStatus.Connecting });
const setStatus = (status: ApiStatus) => setState('status', status);

let messageQueue: Array<ArrayBuffer> = [];

export { state as api };
export const request = (request: Request, retry = true) => {
	const message = encodeRequest(request);
	if (retry) {
		messageQueue.push(message);
	}

	if (socket?.readyState === WebSocket.OPEN) {
		socket.send(message);
	}
};

export function initApi() {
	newSocket();
	onCleanup(() => {
		if (
			socket?.readyState === WebSocket.CONNECTING ||
			socket?.readyState === WebSocket.OPEN
		) {
			socket.close();
		}
	});
}

// Slightly longer than the Status message interval
const messageTimeout = 1500;
const maxReconnectAttempts = 20;

let reconnectAttempt = 0;
let reconnectHandle: ReturnType<typeof setTimeout>;
function reconnect() {
	if (socket) {
		socket.onclose = null;
		socket.close();
	}

	if (++reconnectAttempt === maxReconnectAttempts) {
		setStatus(ApiStatus.LostConnection);
		return;
	}

	newSocket();
}

let watchdogHandle: ReturnType<typeof setTimeout>;
function watchdog() {
	console.warn('API socket timed out');
	setStatus(ApiStatus.Reconnecting);
	reconnect();
}

function newSocket() {
	socket = new WebSocket(`ws://${API_BASE}`, 'v0');

	socket.onopen = () => {
		setStatus(ApiStatus.Connected);
		clearTimeout(reconnectHandle);
		watchdogHandle = setTimeout(watchdog, messageTimeout);
		reconnectAttempt = 0;

		if (socket) {
			for (const message of messageQueue) {
				socket.send(message);
			}
		}
	};

	socket.onclose = () => {
		setStatus(ApiStatus.Reconnecting);
		clearTimeout(watchdogHandle);
		reconnect();
	};

	socket.onmessage = async ({ data }: MessageEvent<string | Blob>) => {
		clearTimeout(watchdogHandle);
		watchdogHandle = setTimeout(watchdog, messageTimeout);
		messageQueue = [];

		if (data instanceof Blob) {
			const response = parseResponse(await data.arrayBuffer());
			setState(response.kind, response.payload);
		}
	};
}
