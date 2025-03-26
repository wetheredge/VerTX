import { z } from 'zod';

const pin = z.number().nonnegative().int();
export const schema = z
	.strictObject({
		chip: z.string(),
		pins: z.strictObject({
			leds: pin,
			sd: pin,
			analog: z.array(pin),
			switches: z.array(pin),
			spi: z.strictObject({
				sclk: pin,
				miso: pin,
				mosi: pin,
			}),
			ui: z.strictObject({
				up: pin,
				down: pin,
				left: pin,
				right: pin,
			}),
			display: z.discriminatedUnion('type', [
				z.strictObject({
					type: z.literal('ssd1306'),
					sda: pin,
					scl: pin,
				}),
			]),
		}),
	})
	.readonly();
