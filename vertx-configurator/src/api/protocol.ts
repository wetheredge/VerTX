import { unreachable } from '../utils';
import { DataReader, DataWriter } from './helpers';

export const PROTOCOL: string = 'v0';
const REQUEST_BUFFER_SIZE = 100;

export const enum RequestKind {
	ProtocolVersion,
	BuildInfo,
	PowerOff,
	Reboot,
	CheckForUpdate,
	ConfigUpdate,
	// StreamInputs,
	// StreamMixer,
}

export type Request =
	| { kind: RequestKind.ProtocolVersion }
	| { kind: RequestKind.BuildInfo }
	| { kind: RequestKind.PowerOff }
	| { kind: RequestKind.Reboot }
	| { kind: RequestKind.CheckForUpdate }
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
		case RequestKind.CheckForUpdate:
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

const enum ConfigUpdateResultKind {
	Ok,
	KeyNotFound,
	InvalidType,
	InvalidValue,
	TooSmall,
	TooLarge,
}

type ConfigUpdateResult =
	| { result: ConfigUpdateResultKind.Ok }
	| { result: ConfigUpdateResultKind.KeyNotFound }
	| { result: ConfigUpdateResultKind.InvalidType }
	| { result: ConfigUpdateResultKind.TooSmall; min: number }
	| { result: ConfigUpdateResultKind.TooLarge; max: number };

export type ResponsePayload<Kind extends ResponseKind> = Extract<
	Response,
	{ kind: Kind; payload: unknown }
>['payload'];

export function parseResponse(buffer: ArrayBuffer): Response {
	const reader = new DataReader(buffer);

	const kind = reader.u8();
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

		default:
			throw new Error(`Invalid response kind: ${kind}`);
	}
}

export type ProtocolVersion = {
	major: number;
	minor: number;
};

export type FirmwareVersion = {
	major: number;
	minor: number;
	patch: number;
	commit: string;
	dirty: boolean;
};
