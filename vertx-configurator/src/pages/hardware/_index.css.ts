import { globalStyle, style } from '@vanilla-extract/css';
import { button } from '../../styles/components.css.ts';
import { mediaIsMobile, size } from '../../styles/constants.ts';

export const calibrate = button;

export const inputs = style({
	display: 'grid',
	gridTemplateColumns: 'min-content auto min-content',
	gap: size.sm,

	'@media': {
		[`not ${mediaIsMobile}`]: {
			columnGap: size.md,
		},
	},
});

globalStyle(`${inputs} > *`, {
	marginBlock: 'auto',
});
