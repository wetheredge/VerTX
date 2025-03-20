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

type Get<Resp = undefined> = {
	method: 'GET';
	response: Resp;
};

type Post<Req = undefined, Resp = undefined> = {
	method: 'POST';
	request: Req;
	response: Resp;
};

type Delete<Resp = undefined> = {
	method: 'DELETE';
	response: Resp;
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
