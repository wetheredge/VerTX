import { z } from 'zod';

export type Target = z.infer<typeof schema>;

const pin = z.number().nonnegative().int();
export const schema = z
	.strictObject({
		chip: z.string(),
		leds: z.strictObject({
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
				sclk: pin,
				miso: pin,
				mosi: pin,
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
	.check((ctx) => {
		const needsSpi = ctx.value.sd.type === 'spi';
		const hasSpi = ctx.value.spi != null;

		if (needsSpi && !hasSpi) {
			ctx.issues.push({
				code: 'invalid_type',
				path: ['spi'],
				expected: 'object',
				input: ctx.value.spi,
			});
		} else if (hasSpi && !needsSpi) {
			ctx.issues.push({
				code: 'custom',
				path: ['spi'],
				message: 'Unused and should be removed',
				input: ctx.value,
			});
		}
	});
