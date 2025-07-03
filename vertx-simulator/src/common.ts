export type Method = 'GET' | 'POST' | 'DELETE';

export type ConfiguratorRequest = {
	vertx: 'request';
	id: number;
	route: string;
	method: Method;
	body: ArrayBuffer | undefined;
};

export type ConfiguratorResponse = {
	vertx: 'response';
	id: number;
	status: number;
	headers: Headers | Record<string, string>;
	body: ArrayBuffer;
};

export function isRequest(x: unknown): x is ConfiguratorRequest {
	return isVertxObj(x) && x.vertx === 'request';
}

export function isResponse(x: unknown): x is ConfiguratorResponse {
	return isVertxObj(x) && x.vertx === 'response';
}

function isVertxObj(
	x: unknown,
): x is ConfiguratorRequest | ConfiguratorResponse {
	return typeof x === 'object' && x !== null && 'vertx' in x;
}
