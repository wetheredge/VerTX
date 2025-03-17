export type Method = 'GET' | 'POST' | 'DELETE';

export type ConfiguratorRequest = {
	id: number;
	route: string;
	method: Method;
};

export type ConfiguratorResponse = {
	id: number;
	status: number;
	headers: Headers | Record<string, string>;
	body: ArrayBuffer;
};
