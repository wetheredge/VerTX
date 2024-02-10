import { makeReconnectingWS } from '@solid-primitives/websocket';
import { Accessor, createSignal, onCleanup } from 'solid-js';

import { dismissToast, showToast } from '~/components/ui/toast';
import {
	type Request,
	parseResponse,
	encodeRequest,
	type Response,
} from './protocol';

export {
	type Request,
	type Response,
	ResponseKind,
	RequestKind,
} from './protocol';

export type Api = {
	isConnected: Accessor<boolean>;
	request: (request: Request) => void;
};

export default function createApi(
	host: string,
	onResponse: (resp: Response) => void,
): Api {
	const [isConnected, setIsConnected] = createSignal(false);
	let connectionToast: number | undefined;
	let everConnected = false;

	const socket = makeReconnectingWS(`ws://${host}/ws`, 'v0', {
		delay: 15_000,
		retries: 5,
	});

	onCleanup(() => {
		if (connectionToast != null) {
			dismissToast(connectionToast);
			socket.close();
		}
	});

	socket.addEventListener('open', () => {
		everConnected = true;
		if (connectionToast != null) dismissToast(connectionToast);
		showToast({
			priority: 'low',
			title: 'Handset connected',
			duration: 3000,
		});
		setIsConnected(true);
	});

	socket.addEventListener('close', () => {
		if (everConnected && connectionToast == null)
			connectionToast = showToast({
				priority: 'low',
				title: 'Handset connection lost',
				description: 'Attempting to reconnect...',
				persistent: true,
			});
	});

	setTimeout(() => {
		if (!everConnected && connectionToast == null) {
			connectionToast = showToast({
				priority: 'low',
				title: 'Connecting...',
				persistent: true,
			});
		}
	}, 300);

	socket.addEventListener(
		'message',
		async ({ data }: MessageEvent<string | Blob>) => {
			if (data instanceof Blob) {
				onResponse(parseResponse(await data.arrayBuffer()));
			}
		},
	);

	return {
		isConnected,
		request(request) {
			socket.send(encodeRequest(request));
		},
	};
}
