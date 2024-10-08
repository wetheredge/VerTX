import { Reader, Writer } from 'postcard';
import { type Config, type Update, encodeUpdate, parseConfig } from '../config';
import { unreachable } from '../utils';

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
			payload: { id: number } & Update;
	  };

export const enum ConfigUpdateKind {
	Boolean,
	String,
	Unsigned,
	// Signed,
	// Float,
}

export function encodeRequest(request: Request): ArrayBuffer {
	const writer = new Writer(REQUEST_BUFFER_SIZE);

	writer.varuint(request.kind);
	switch (request.kind) {
		case RequestKind.ProtocolVersion:
		case RequestKind.BuildInfo:
		case RequestKind.PowerOff:
		case RequestKind.Reboot:
		case RequestKind.ExitConfigurator:
		case RequestKind.CheckForUpdate:
		case RequestKind.GetConfig:
			break;

		case RequestKind.ConfigUpdate:
			writer.varuint(request.payload.id);
			encodeUpdate(writer, request.payload);
			break;

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

export enum ConfigUpdateResultKind {
	Ok,
	TooSmall,
	TooLarge,
}

export type ConfigUpdateResult =
	| { result: ConfigUpdateResultKind.Ok }
	| { result: ConfigUpdateResultKind.TooSmall; min: number }
	| { result: ConfigUpdateResultKind.TooLarge; max: number };

export function configUpdateResultToString(result: ConfigUpdateResult): string {
	const name = ConfigUpdateResultKind[result.result];
	switch (result.result) {
		case ConfigUpdateResultKind.Ok:
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

export function parseResponse(buffer: DataView): Response {
	const reader = new Reader(buffer);

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
					batteryVoltage: reader.varuint() / 100,
					idleTime: reader.f32(),
					timingDrift: reader.f32(),
				},
			};
		case ResponseKind.Config:
			reader.varuint(); // Ignore config byte array length
			return {
				kind,
				payload: parseConfig(reader),
			};
		case ResponseKind.ConfigUpdate: {
			const id = reader.varuint();
			const result = reader.u8() as ConfigUpdateResultKind;
			const rest: Record<string, unknown> = {};
			switch (result) {
				case ConfigUpdateResultKind.Ok:
					break;
				case ConfigUpdateResultKind.TooSmall:
					rest.min = reader.varuint();
					break;
				case ConfigUpdateResultKind.TooLarge:
					rest.max = reader.varuint();
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
