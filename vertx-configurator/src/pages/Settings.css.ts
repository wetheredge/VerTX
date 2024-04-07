import { globalStyle, style } from '@vanilla-extract/css';
import { fontSize, mediaIsMobile, space, vars } from '../index.css';

export const setting = style({
	display: 'flex',
	flexDirection: 'column',
	gap: space.sm,
});
globalStyle(`${setting} > label`, {
	fontWeight: 500,
});
globalStyle(`${setting} > :is(input, select)`, {
	width: '25ch',

	border: vars.border,
	borderRadius: space.sm,
	color: 'inherit',
	background: vars.colors.bgInput,
	padding: `${space.xs} ${space.sm}`,
	outline: 'none',

	'@media': {
		[mediaIsMobile]: {
			width: '100%',
		},
	},
});
globalStyle(`${setting} > ${':is(input, select)'}:focus-visible`, {
	borderColor: vars.colors.borderFocus,
});
globalStyle(`${setting} > span`, {
	fontSize: fontSize.small,
	color: vars.colors.fgDim,
});
