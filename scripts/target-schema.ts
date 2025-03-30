import { z } from 'zod';

export type Target = z.infer<typeof schema>;

const index = z.number().nonnegative().int();
const name = z.string().nonempty();
const pin = z.union([index, name]);
export const schema = z
	.strictObject({
		chip: z.string(),
		leds: z.strictObject({
			timer: name.optional(),
			dma: name.optional(),
			status: pin,
		}),
		sd: z.discriminatedUnion('type', [
			z.strictObject({
				type: z.literal('spi'),
				cs: pin,
			}),
		]),
		spi: z
			.strictObject({
				peripheral: name.optional(),
				sclk: pin,
				miso: pin,
				mosi: pin,
				dma: z.strictObject({ tx: name, rx: name }).optional(),
			})
			.optional(),
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
	})
	.readonly()
	.superRefine((val, ctx) => {
		const needsSpi = val.sd.type === 'spi';
		const hasSpi = val.spi != null;

		if (needsSpi && !hasSpi) {
			ctx.addIssue({
				code: z.ZodIssueCode.invalid_type,
				path: [...ctx.path, 'spi'],
				expected: 'object',
				received: 'undefined',
			});
		} else if (hasSpi && !needsSpi) {
			ctx.addIssue({
				code: z.ZodIssueCode.unrecognized_keys,
				path: ctx.path,
				keys: ['spi'],
				message: '.spi is unused',
			});
		}
	});
