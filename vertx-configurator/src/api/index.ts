import type {
	ConfiguratorRequest,
	ConfiguratorResponse,
	Method,
} from './common';
import type { RoutesFor } from './types';

type Accept = 'json' | 'binary';
type Headers = NonNullable<NonNullable<Parameters<typeof fetch>[1]>['headers']>;

const isSimulator = import.meta.env.VERTX_TARGET === 'simulator';
let simulatorRequestId = 0;
const simulatorPromises = new Map<number, (resp: Response) => void>();

function fetchNative(
	method: Method,
	route: string,
	headers: Headers,
): Promise<Response> {
	return fetch(`/api/${route}`, { method, headers });
}

async function fetchSimulator(
	method: Method,
	route: string,
): Promise<Response> {
	const request: ConfiguratorRequest = {
		id: simulatorRequestId++,
		route,
		method,
	};
	// FIXME: wildcard origin
	window.opener.postMessage(request, '*');

	return new Promise((resolve) => {
		simulatorPromises.set(request.id, resolve);
	});
}

if (isSimulator) {
	window.addEventListener(
		'message',
		(event: MessageEvent<ConfiguratorResponse>) => {
			const { id, body, ...init } = event.data;
			const resolve = simulatorPromises.get(id);
			if (resolve) {
				resolve(new Response(body, init));
			}
		},
	);
}

async function request<T>(
	method: Method,
	route: string,
	accept: 'json',
): Promise<T>;
async function request(
	method: Method,
	route: string,
	accept: 'binary',
): Promise<ArrayBuffer>;
async function request(method: Method, route: string): Promise<void>;
async function request<T>(
	method: Method,
	route: string,
	accept?: Accept,
): Promise<T | ArrayBuffer | undefined> {
	const mimes: Record<Accept, string> = {
		json: 'application/json',
		binary: 'application/octet-stream',
	};

	const headers = {
		// biome-ignore lint/style/useNamingConvention:
		Accept: accept ? mimes[accept] : '*/*',
	};

	const fetch = isSimulator ? fetchSimulator : fetchNative;
	const response = await fetch(method, route, headers);
	if (!response.ok) {
		throw new ApiError(route, response);
	}

	switch (accept) {
		case 'json':
			return response.json();
		case 'binary':
			return response.arrayBuffer();
	}
}

export const getJson = <
	Routes extends RoutesFor<'GET', 'json'>,
	P extends Routes['path'],
>(
	route: P,
) => request<Extract<Routes, { path: P }>['response']>('GET', route, 'json');

export const getBinary = (route: RoutesFor<'GET', 'binary'>['path']) =>
	request('GET', route, 'binary');

export const post = (route: RoutesFor<'POST'>['path']) =>
	request('POST', route);

export const postJson = <
	Routes extends RoutesFor<'POST', 'json'>,
	P extends Routes['path'],
>(
	route: P,
) => request<Extract<Routes, { path: P }>['response']>('POST', route, 'json');

export const postBinary = (route: RoutesFor<'POST', 'binary'>['path']) =>
	request('POST', route, 'binary');

const delete_ = (route: RoutesFor<'DELETE'>['path']) =>
	request('DELETE', route);
export { delete_ as delete };

export class ApiError extends Error {
	name = 'ApiError';
	route: string;
	response: Response;

	constructor(route: string, resp: Response, ...params: Array<ErrorOptions>) {
		super(`API request to '${route}' failed: ${resp.status}`, ...params);

		Error.captureStackTrace?.(this, ApiError);

		this.route = route;
		this.response = resp;
	}
}
