import { unreachable } from '../utils';
import { DataReader, DataWriter } from './helpers';

export const PROTOCOL: string = 'v0';
const REQUEST_BUFFER_SIZE = 2;

export const enum RequestKind {
	ProtocolVersion,
	BuildInfo,
	PowerOff,
	Reboot,
	CheckForUpdate,
	StreamInputs,
	StreamOutputs,
}

export type Request =
	| { kind: RequestKind.ProtocolVersion }
	| { kind: RequestKind.BuildInfo }
	| { kind: RequestKind.PowerOff }
	| { kind: RequestKind.Reboot }
	| { kind: RequestKind.CheckForUpdate }
	| { kind: RequestKind.StreamInputs; payload: boolean }
	| { kind: RequestKind.StreamOutputs; payload: boolean };

export function encodeRequest(request: Request): ArrayBuffer | DataView {
	const writer = new DataWriter(REQUEST_BUFFER_SIZE);

	writer.varint(request.kind);
	switch (request.kind) {
		case RequestKind.ProtocolVersion:
		case RequestKind.BuildInfo:
		case RequestKind.PowerOff:
		case RequestKind.Reboot:
		case RequestKind.CheckForUpdate:
			break;
		case RequestKind.StreamInputs:
		case RequestKind.StreamOutputs:
			writer.boolean(request.payload);
			break;

		default:
			unreachable(request);
	}

	return writer.done();
}

export enum ResponseKind {
	ProtocolVersion,
	BuildInfo,
	Status,
	Inputs,
	Outputs,
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
			kind: ResponseKind.Inputs;
			payload: Array<number>;
	  }
	| {
			kind: ResponseKind.Outputs;
			payload: Array<number>;
	  };

export type ResponsePayload<Kind extends ResponseKind> = Extract<
	Response,
	{ kind: Kind }
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
		case ResponseKind.Inputs:
			return {
				kind,
				payload: Array.from({ length: reader.varint() }).map(() =>
					reader.varint(),
				),
			};
		case ResponseKind.Outputs:
			return {
				kind,
				payload: Array.from({ length: 16 }).map(() => reader.varint()),
			};

		default:
			unreachable(kind, `Invalid response kind: ${kind}`);
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
