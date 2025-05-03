import { z } from 'zod';

const type = Symbol();

const index = z.number().nonnegative().int();
const name = z.string().nonempty();
const pin = z.union([index, name]).transform((pin) => ({ [type]: 'pin', pin }));
export type Pin = z.infer<typeof pin>;
export function isPin(x: Pin | object): x is Pin {
	return type in x && x[type] === 'pin';
}

const dmaPair = z.strictObject({ tx: name, rx: name });

export type Target = z.infer<typeof schema>;
export const schema = z
	.strictObject({
		chip: z.string(),
		leds: z.strictObject({
			timer: name.optional(),
			dma: name.optional(),
			status: z.union([
				pin,
				z.strictObject({
					red: pin,
					green: pin,
					blue: pin,
				}),
			]),
		}),
		sd: z.discriminatedUnion('type', [
			z.strictObject({
				type: z.literal('spi'),
				spi: name.optional(),
				dma: dmaPair.optional(),
				cs: pin,
				sclk: pin,
				miso: pin,
				mosi: pin,
			}),
		]),
		ui: z.strictObject({
			up: pin,
			down: pin,
			left: pin,
			right: pin,
		}),
		display: z.discriminatedUnion('driver', [
			z.strictObject({
				driver: z.literal('ssd1306'),
				i2c: name.optional(),
				dma: dmaPair.optional(),
				sda: pin,
				scl: pin,
			}),
		]),
	})
	.readonly();
