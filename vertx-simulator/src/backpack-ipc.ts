import { Reader, Writer } from 'postcard';

// b"VerTX\0"
export const INIT = [86, 101, 114, 84, 88, 0];

export const enum NetworkKind {
	Home,
	Field,
}

type NetworkConfigBase<Kind extends NetworkKind> = {
	network: Kind;
	ssid: string;
	password: string;
};
export type NetworkConfig<Kind extends NetworkKind> = NetworkConfigBase<Kind> &
	(Kind extends NetworkKind.Home
		? { hostname: string }
		: Kind extends NetworkKind.Field
			? { address: string }
			: never);
type AnyNetworkConfig = NetworkConfig<NetworkKind>;

export const enum ToBackpackKind {
	SetBootMode,
	StartNetwork,
	ApiResponse,
	ShutDown,
	Reboot,
}

export type ToBackpack =
	| { kind: ToBackpackKind; payload: number }
	| { kind: ToBackpackKind.StartNetwork; payload: AnyNetworkConfig }
	| { kind: ToBackpackKind.ApiResponse; payload: Uint8Array }
	| { kind: ToBackpackKind.ShutDown }
	| { kind: ToBackpackKind.Reboot };

export function decode(data: DataView): ToBackpack {
	const reader = new Reader(data);

	const kind = reader.u8() as ToBackpackKind;
	switch (kind) {
		case ToBackpackKind.SetBootMode:
			return { kind, payload: reader.u8() };

		case ToBackpackKind.StartNetwork: {
			const networkKind = reader.u8() as NetworkKind;
			if (networkKind <= NetworkKind.Field) {
				const base: NetworkConfigBase<typeof networkKind> = {
					network: networkKind,
					ssid: reader.string(),
					password: reader.string(),
				};
				return {
					kind,
					payload:
						networkKind === NetworkKind.Home
							? { ...base, hostname: reader.string() }
							: {
									...base,
									address: Array.from({ length: 4 })
										.map(() => reader.u8())
										.join('.'),
								},
				};
			}
			throw new Error(`Invalid NetworkConfig kind: ${networkKind}`);
		}

		case ToBackpackKind.ApiResponse:
			return { kind, payload: reader.byteArray() };

		case ToBackpackKind.ShutDown:
		case ToBackpackKind.Reboot:
			return { kind } as
				| { kind: ToBackpackKind.ShutDown }
				| { kind: ToBackpackKind.Reboot };

		default:
			throw new Error(`Invalid ToBackpack kind: ${kind}`);
	}
}

export const enum ToMainKind {
	NetworkUp,
	ApiRequest,
	PowerAck,
}

export type ToMain =
	| { kind: ToMainKind.NetworkUp }
	| { kind: ToMainKind.ApiRequest; payload: Uint8Array }
	| { kind: ToMainKind.PowerAck };

export function encode(message: ToMain): Uint8Array {
	const writer = new Writer(256);

	writer.varint(message.kind);
	switch (message.kind) {
		case ToMainKind.NetworkUp:
		case ToMainKind.PowerAck:
			break;

		case ToMainKind.ApiRequest:
			writer.byteArray(message.payload);
			break;
	}

	return new Uint8Array(writer.done());
}
