import { makeReconnectingWS } from '@solid-primitives/websocket';
import { onCleanup } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { Update } from '../config';
import { isSimulator } from '../utils';
import {
	type ConfigUpdateResult,
	ConfigUpdateResultKind,
	type Request,
	RequestKind,
	ResponseKind,
	type ResponsePayload,
	encodeRequest,
	parseResponse,
} from './protocol';

export {
	type ConfigUpdateResult,
	ConfigUpdateResultKind,
	type Request,
	type Response,
	type ResponsePayload,
	ResponseKind,
	RequestKind,
	configUpdateResultToString,
} from './protocol';

export const enum ApiStatus {
	Connecting,
	Connected,
	NotConnected,
}

type State = { status: ApiStatus } & {
	[Kind in Exclude<
		ResponseKind,
		ResponseKind.ConfigUpdate
	>]?: ResponsePayload<Kind>;
};

type Sender = (request: ArrayBuffer) => void;
let resolveSender!: (sender: Sender) => void;
const sender = new Promise<Sender>((resolve) => {
	resolveSender = resolve;
});
const [state, setState] = createStore<State>({
	status: ApiStatus.Connecting,
});
const setStatus = (status: ApiStatus) => setState('status', status);

export { state as api };
export const request = (request: Request) => {
	import.meta.env.DEV && console.debug('request', request);
	const encoded = encodeRequest(request);
	sender.then((send) => send(encoded));
};

const configUpdates = new Map<number, (result: ConfigUpdateResult) => void>();
let updateId = Date.now() & 0xffff;
export async function updateConfig(
	update: Exclude<Update, { key: never }>,
): Promise<ConfigUpdateResult> {
	const id = updateId;
	updateId = (updateId + 1) >>> 0;

	request({
		kind: RequestKind.UpdateConfig,
		payload: { id, ...update },
	});

	const result = await new Promise<ConfigUpdateResult>((resolve) => {
		configUpdates.set(id, resolve);
	});
	if (result.result === ConfigUpdateResultKind.Ok) {
		setState(ResponseKind.Config, update.key, update.value);
	}
	return result;
}

function handleResponse(data: ArrayBuffer) {
	const response = parseResponse(new DataView(data));
	import.meta.env.DEV && console.debug('response', response);
	if (response.kind === ResponseKind.Config) {
		setState(ResponseKind.Config, response.payload);
	} else if (response.kind === ResponseKind.ConfigUpdate) {
		const { id, ...result } = response.payload;
		configUpdates.get(id)?.(result);
		configUpdates.delete(id);
	} else {
		setState(response.kind, response.payload);
	}
}

function initNative() {
	const socket = makeReconnectingWS(`ws://${location.host}/api`, 'v0', {
		delay: 15_000,
		retries: 5,
	});

	onCleanup(() => socket.close());

	socket.addEventListener('open', () => setStatus(ApiStatus.Connected));
	socket.addEventListener('close', () => setStatus(ApiStatus.NotConnected));

	socket.addEventListener(
		'message',
		async ({ data }: MessageEvent<string | Blob>) => {
			if (data instanceof Blob) {
				handleResponse(await data.arrayBuffer());
			}
		},
	);

	resolveSender((request) => socket.send(request));
}

function initSimulator() {
	const opener = window.opener as WindowProxy | null;

	if (opener?.origin !== location.origin) {
		const message =
			'This build of the configurator can only run from the simulator';
		alert(message);
		throw new Error(message);
	}

	setStatus(ApiStatus.Connected);
	opener.addEventListener('close', () => setStatus(ApiStatus.NotConnected));

	window.addEventListener('message', (event) => {
		if (
			event.origin !== location.origin ||
			!(event.data instanceof ArrayBuffer)
		) {
			return;
		}

		handleResponse(event.data);
	});

	resolveSender((request) => opener.postMessage(request));
}

export const init = isSimulator ? initSimulator : initNative;
