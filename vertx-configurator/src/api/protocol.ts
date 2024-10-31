import { Reader, Writer } from 'postcard';
import { type Config, type Update, encodeUpdate, parseConfig } from '../config';
import { unreachable } from '../utils';

export const PROTOCOL: string = 'v0';
const REQUEST_BUFFER_SIZE = 100;

export const enum RequestKind {
	BuildInfo,
	PowerOff,
	Reboot,
	ExitConfigurator,
	GetConfig,
	UpdateConfig,
	// StreamInputs,
	// StreamMixer,
}

export type Request =
	| { kind: RequestKind.BuildInfo }
	| { kind: RequestKind.PowerOff }
	| { kind: RequestKind.Reboot }
	| { kind: RequestKind.ExitConfigurator }
	| { kind: RequestKind.GetConfig }
	| {
			kind: RequestKind.UpdateConfig;
			payload: { id: number } & Update;
	  };

export function encodeRequest(request: Request): ArrayBuffer {
	const writer = new Writer(REQUEST_BUFFER_SIZE);

	writer.varuint(request.kind);
	switch (request.kind) {
		case RequestKind.BuildInfo:
		case RequestKind.PowerOff:
		case RequestKind.Reboot:
		case RequestKind.ExitConfigurator:
		case RequestKind.GetConfig:
			break;

		case RequestKind.UpdateConfig:
			writer.varuint(request.payload.id);
			encodeUpdate(writer, request.payload);
			break;

		default:
			unreachable(request);
	}

	return writer.done();
}

export const enum ResponseKind {
	BuildInfo,
	Vbat,
	Config,
	ConfigUpdate,
}

export type Response =
	| {
			kind: ResponseKind.BuildInfo;
			payload: {
				target: string;
				version: string;
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
			kind: ResponseKind.Vbat;
			payload: number;
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
		case ResponseKind.BuildInfo:
			return {
				kind,
				payload: {
					target: reader.string(),
					version: reader.string(),
					debug: reader.boolean(),
					git: {
						branch: reader.string(),
						commit: reader.string(),
						dirty: reader.boolean(),
					},
				},
			};
		case ResponseKind.Vbat:
			return {
				kind,
				payload: reader.varuint() / 100,
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
