import { makeReconnectingWS } from '@solid-primitives/websocket';
import { onCleanup } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { Update } from '../config';
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
	ConfigUpdateKind,
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

let socket!: WebSocket;
const [state, setState] = createStore<State>({
	status: ApiStatus.Connecting,
});
const setStatus = (status: ApiStatus) => setState('status', status);

export { state as api };
export const request = (request: Request) => {
	import.meta.env.DEV && console.debug('request', request);
	socket.send(encodeRequest(request));
};

const configUpdates = new Map<number, (result: ConfigUpdateResult) => void>();
let updateId = Date.now() & 0xffff;
export async function updateConfig(
	update: Update,
): Promise<ConfigUpdateResult> {
	const id = updateId;
	updateId = (updateId + 1) >>> 0;

	request({
		kind: RequestKind.ConfigUpdate,
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

export function initApi(host: string) {
	socket = makeReconnectingWS(`ws://${host}/api`, 'v0', {
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
				const response = parseResponse(
					new DataView(await data.arrayBuffer()),
				);
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
		},
	);
}
