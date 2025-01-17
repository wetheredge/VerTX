import { globalStyle, style } from '@vanilla-extract/css';
import { inputBase } from '../../styles/components.css.ts';
import { fontSize, mediaIsMobile, size } from '../../styles/constants.ts';
import { theme } from '../../styles/theme.css.ts';

const gap = size.sm;

export const setting = style({
	display: 'flex',
	flexDirection: 'column',
	gap,
});

export const input = style([
	inputBase,
	{
		width: '25ch',
		padding: `${size.xs} ${size.sm}`,
		color: 'inherit',

		'@media': {
			[mediaIsMobile]: {
				width: '100%',
			},
		},
	},
]);

globalStyle(`${setting} > label`, {
	fontWeight: 500,
});
globalStyle(`${setting} > span`, {
	fontSize: fontSize.small,
	color: theme.color.fgDim,
});
