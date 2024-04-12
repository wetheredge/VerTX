import { globalStyle, style } from '@vanilla-extract/css';
import { space, vars } from '../index.css';

export const localUpdate = style({
	cursor: 'pointer',
	width: 'fit-content',
	border: vars.border,
	borderRadius: space.sm,
	background: vars.colors.bgInput,
	padding: `${space.xs} ${space.sm}`,
	outline: 'none',
});
globalStyle(`${localUpdate} > input`, {
	display: 'none',
});
