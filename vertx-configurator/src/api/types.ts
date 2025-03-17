export type Version = {
	target: string;
	version: string;
	release: boolean;
	git: {
		branch: string;
		commit: string;
	};
};

export type Model = {
	id: string;
	name: string;
};

type Get<T = undefined> = {
	method: 'GET';
	response: T;
};

type Post<Body = undefined, T = undefined> = {
	method: 'POST';
	request: Body;
	response: T;
};

type Delete<T = undefined> = {
	method: 'DELETE';
	response: T;
};

type RoutesRaw = {
	version: Get<Version>;
	reboot: Post;
	'shut-down': Post;
	config: Get<ArrayBuffer> | Post<ArrayBuffer> | Delete;
	models: Get<Array<Model>>;
};
type RoutesMap = {
	[Path in keyof RoutesRaw]: { path: Path } & RoutesRaw[Path];
};
type Routes = RoutesMap[keyof RoutesMap];

export type RoutesFor<
	M extends Routes['method'],
	T extends 'json' | 'binary' | undefined = undefined,
> = T extends undefined
	? Extract<Routes, { method: M }>
	: T extends 'json'
		? Exclude<RoutesFor<M>, { response: ArrayBuffer }>
		: Extract<RoutesFor<M>, { response: ArrayBuffer }>;
