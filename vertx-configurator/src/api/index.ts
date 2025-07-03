import { VERTX_SIMULATOR } from 'astro:env/client';
import { type ConfiguratorRequest, isResponse, type Method } from './common.ts';
import type { RoutesFor } from './types.ts';

type Accept = 'json' | 'binary';
type Headers = NonNullable<NonNullable<Parameters<typeof fetch>[1]>['headers']>;
type Body = ArrayBuffer | undefined;
type ToString = { toString(): string };

let simulatorRequestId = 0;
const simulatorPromises = new Map<number, (resp: Response) => void>();

function fetchNative(
	method: Method,
	route: string,
	headers: Headers,
	body: Body,
): Promise<Response> {
	return fetch(`/api/${route}`, { method, headers, body });
}

function fetchSimulator(
	method: Method,
	route: string,
	_headers: Headers,
	body: Body,
): Promise<Response> {
	const request: ConfiguratorRequest = {
		vertx: 'request',
		id: simulatorRequestId++,
		route,
		method,
		body,
	};

	if (import.meta.env.DEV) {
		window.opener.postMessage(request, '*');
	} else {
		window.opener.postMessage(request);
	}

	return new Promise((resolve) => {
		simulatorPromises.set(request.id, resolve);
	});
}

if (VERTX_SIMULATOR) {
	window.addEventListener('message', (event) => {
		if (
			!isResponse(event.data) ||
			(import.meta.env.PROD && event.origin !== location.origin)
		) {
			return;
		}

		const { id, body, ...init } = event.data;
		const resolve = simulatorPromises.get(id);
		if (resolve) {
			resolve(new Response(body, init));
		}
	});
}

async function request<T>(
	method: Method,
	route: string,
	query: Record<string, ToString>,
	body: ArrayBuffer | undefined,
	accept: 'json',
): Promise<T>;
async function request(
	method: Method,
	route: string,
	query: Record<string, ToString>,
	body: ArrayBuffer | undefined,
	accept: 'binary',
): Promise<ArrayBuffer>;
async function request(
	method: Method,
	route: string,
	query: Record<string, ToString>,
	body?: ArrayBuffer,
): Promise<void>;
async function request<T>(
	method: Method,
	route: string,
	query: Record<string, ToString>,
	body?: ArrayBuffer,
	accept?: Accept,
): Promise<T | ArrayBuffer | undefined> {
	const mimes: Record<Accept, string> = {
		json: 'application/json',
		binary: 'application/octet-stream',
	};

	const headers = {
		accept: accept ? mimes[accept] : '*/*',
	};

	let fullRoute = route;
	const queryEntries = Object.entries(query);
	for (let i = 0; i < queryEntries.length; i++) {
		fullRoute += i === 0 ? '?' : '&';
		// biome-ignore lint/style/noNonNullAssertion: for loop condition keeps i in bounds
		fullRoute += queryEntries[i]!.map((s) =>
			encodeURIComponent(s.toString()),
		).join('=');
	}

	const fetch = VERTX_SIMULATOR ? fetchSimulator : fetchNative;
	const response = await fetch(method, fullRoute, headers, body);
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

type MaybeQuery<R> = 'query' extends keyof R ? [R['query']] : [];

export const getJson = <
	Routes extends RoutesFor<'GET', 'json'>,
	P extends Routes['path'],
>(
	route: P,
	...[query]: MaybeQuery<Extract<Routes, { path: P }>>
) =>
	request<Extract<Routes, { path: P }>['response']>(
		'GET',
		route,
		query ?? {},
		undefined,
		'json',
	);

export const getBinary = (route: RoutesFor<'GET', 'binary'>['path']) =>
	request('GET', route, {}, undefined, 'binary');

type MaybeBody<T> = T extends undefined ? [] : [T];

export const post = <
	Routes extends RoutesFor<'POST'>,
	P extends Routes['path'],
>(
	route: P,
	...[body]: MaybeBody<Extract<Routes, { path: P }>['request']>
) => request('POST', route, {}, body);

export const postJson = <
	Routes extends RoutesFor<'POST', 'json'>,
	P extends Routes['path'],
>(
	route: P,
	...[body]: MaybeBody<Extract<Routes, { path: P }>['request']>
) =>
	request<Extract<Routes, { path: P }>['response']>(
		'POST',
		route,
		{},
		body,
		'json',
	);

export const postBinary = <
	Routes extends RoutesFor<'POST', 'binary'>,
	P extends Routes['path'],
>(
	route: P,
	...[body]: MaybeBody<Extract<Routes, { path: P }>['request']>
) => request('POST', route, {}, body, 'binary');

const delete_ = (route: RoutesFor<'DELETE'>['path']) =>
	request('DELETE', route, {});
export { delete_ as delete };

export class ApiError extends Error {
	override name = 'ApiError';
	route: string;
	response: Response;

	constructor(route: string, resp: Response, ...params: Array<ErrorOptions>) {
		super(`API request to '${route}' failed: ${resp.status}`, ...params);

		Error.captureStackTrace?.(this, ApiError);

		this.route = route;
		this.response = resp;
	}
}
