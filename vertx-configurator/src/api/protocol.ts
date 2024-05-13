import { unreachable } from '../utils';
import { DataReader, DataWriter } from './helpers';

export const PROTOCOL: string = 'v0';
const REQUEST_BUFFER_SIZE = 100;

export const enum RequestKind {
	ProtocolVersion,
	BuildInfo,
	PowerOff,
	Reboot,
	ExitConfigurator,
	CheckForUpdate,
	GetConfig,
	ConfigUpdate,
	// StreamInputs,
	// StreamMixer,
}

export type Request =
	| { kind: RequestKind.ProtocolVersion }
	| { kind: RequestKind.BuildInfo }
	| { kind: RequestKind.PowerOff }
	| { kind: RequestKind.Reboot }
	| { kind: RequestKind.ExitConfigurator }
	| { kind: RequestKind.CheckForUpdate }
	| { kind: RequestKind.GetConfig }
	| {
			kind: RequestKind.ConfigUpdate;
			payload: {
				id: number;
				key: string;
			} & ConfigUpdate;
	  };

export const enum ConfigUpdateKind {
	Boolean,
	String,
	Unsigned,
	// Signed,
	// Float,
}

export type ConfigUpdate =
	| { kind: ConfigUpdateKind.Boolean; update: boolean }
	| { kind: ConfigUpdateKind.String; update: string }
	| { kind: ConfigUpdateKind.Unsigned; update: number };

export function encodeRequest(request: Request): ArrayBuffer {
	const writer = new DataWriter(REQUEST_BUFFER_SIZE);

	writer.varint(request.kind);
	switch (request.kind) {
		case RequestKind.ProtocolVersion:
		case RequestKind.BuildInfo:
		case RequestKind.PowerOff:
		case RequestKind.Reboot:
		case RequestKind.ExitConfigurator:
		case RequestKind.CheckForUpdate:
		case RequestKind.GetConfig:
			break;

		case RequestKind.ConfigUpdate: {
			const { id, key, kind, update } = request.payload;
			writer.varint(id);
			writer.string(key);
			writer.u8(kind);
			switch (kind) {
				case ConfigUpdateKind.Boolean:
					writer.boolean(update);
					break;
				case ConfigUpdateKind.String:
					writer.string(update);
					break;
				case ConfigUpdateKind.Unsigned:
					writer.varint(update);
					break;
				default:
					unreachable(kind);
			}
			break;
		}

		default:
			unreachable(request);
	}

	return writer.done();
}

export const enum ResponseKind {
	ProtocolVersion,
	BuildInfo,
	Status,
	Config,
	ConfigUpdate,
}

export type Response =
	| {
			kind: ResponseKind.ProtocolVersion;
			payload: {
				major: number;
				minor: number;
			};
	  }
	| {
			kind: ResponseKind.BuildInfo;
			payload: {
				target: string;
				major: number;
				minor: number;
				patch: number;
				suffix: string;
				debug: boolean;
				git: {
					branch: string;
					commit: string;
					dirty: boolean;
				};
			};
	  }
	| {
			kind: ResponseKind.Config;
			payload: Config;
	  }
	| {
			kind: ResponseKind.Status;
			payload: {
				batteryVoltage: number;
				idleTime: number;
				timingDrift: number;
			};
	  }
	| {
			kind: ResponseKind.ConfigUpdate;
			payload: { id: number } & ConfigUpdateResult;
	  };

export type Config = Record<string, boolean | number | string>;

export enum ConfigUpdateResultKind {
	Ok,
	KeyNotFound,
	InvalidType,
	InvalidValue,
	TooSmall,
	TooLarge,
}

export type ConfigUpdateResult =
	| { result: ConfigUpdateResultKind.Ok }
	| { result: ConfigUpdateResultKind.KeyNotFound }
	| { result: ConfigUpdateResultKind.InvalidType }
	| { result: ConfigUpdateResultKind.TooSmall; min: number }
	| { result: ConfigUpdateResultKind.TooLarge; max: number };

export function configUpdateResultToString(result: ConfigUpdateResult): string {
	const name = ConfigUpdateResultKind[result.result];
	switch (result.result) {
		case ConfigUpdateResultKind.Ok:
		case ConfigUpdateResultKind.KeyNotFound:
		case ConfigUpdateResultKind.InvalidType:
			return name;
		case ConfigUpdateResultKind.TooSmall:
			return `${name} { min: ${result.min} }`;
		case ConfigUpdateResultKind.TooLarge:
			return `${name} { max: ${result.max} }`;
		default:
			unreachable(result);
	}
}

export type ResponsePayload<Kind extends ResponseKind> = Extract<
	Response,
	{ kind: Kind; payload: unknown }
>['payload'];

export function parseResponse(buffer: ArrayBuffer): Response {
	const reader = new DataReader(buffer);

	const kind = reader.u8() as ResponseKind;
	switch (kind) {
		case ResponseKind.ProtocolVersion:
			return {
				kind,
				payload: { major: reader.u8(), minor: reader.u8() },
			};
		case ResponseKind.BuildInfo:
			return {
				kind,
				payload: {
					target: reader.string(),
					major: reader.u8(),
					minor: reader.u8(),
					patch: reader.u8(),
					suffix: reader.string(),
					debug: reader.boolean(),
					git: {
						branch: reader.string(),
						commit: reader.string(),
						dirty: reader.boolean(),
					},
				},
			};
		case ResponseKind.Status:
			return {
				kind,
				payload: {
					batteryVoltage: reader.varint() / 100,
					idleTime: reader.f32(),
					timingDrift: reader.f32(),
				},
			};
		case ResponseKind.Config:
			reader.varint(); // Ignore config byte array length
			if (reader.u8() !== ConfigKind.Struct) {
				throw new Error('Invalid config');
			}
			return {
				kind,
				payload: parseConfig(reader),
			};
		case ResponseKind.ConfigUpdate: {
			const id = reader.varint();
			const result = reader.u8() as ConfigUpdateResultKind;
			const rest: Record<string, unknown> = {};
			switch (result) {
				case ConfigUpdateResultKind.Ok:
				case ConfigUpdateResultKind.KeyNotFound:
				case ConfigUpdateResultKind.InvalidType:
				case ConfigUpdateResultKind.InvalidValue:
					break;
				case ConfigUpdateResultKind.TooSmall:
					rest.min = reader.varint();
					break;
				case ConfigUpdateResultKind.TooLarge:
					rest.max = reader.varint();
					break;

				default:
					unreachable(result);
			}

			return {
				kind,
				payload: {
					id,
					result,
					...rest,
				} as ResponsePayload<ResponseKind.ConfigUpdate>,
			};
		}

		default:
			invalidResponseKind(kind);
	}
}

function invalidResponseKind(kind: never): never {
	throw new Error(`Invalid response kind: ${kind}`);
}

const enum ConfigKind {
	Boolean,
	String,
	Unsigned,
	Signed,
	Float,
	Struct,
}

function invalidConfigKind(kind: never): never {
	throw new Error(`Invalid config kind: ${kind}`);
}

function parseConfig(
	reader: DataReader,
	config: Config = {},
	root?: string,
): Config {
	const path = root ? `${root}.` : '';
	for (let i = reader.varint(); i > 0; i--) {
		const field = `${path}${reader.string()}`;
		const kind = reader.u8() as ConfigKind;
		switch (kind) {
			case ConfigKind.Boolean:
				config[field] = reader.boolean();
				break;
			case ConfigKind.String:
				config[field] = reader.string();
				break;
			case ConfigKind.Unsigned:
				config[field] = reader.varint();
				break;
			case ConfigKind.Signed:
				throw new Error('unimplemented');
			case ConfigKind.Float:
				config[field] = reader.f32();
				break;
			case ConfigKind.Struct:
				parseConfig(reader, config, field);
				break;

			default:
				invalidConfigKind(kind);
		}
	}

	return config;
}
