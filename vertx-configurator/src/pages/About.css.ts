import { globalStyle, style } from '@vanilla-extract/css';
import { consts, vars } from '../index.css';

export const buildInfoTable = style({
	borderCollapse: 'collapse',
	borderSpacing: 0,
	border: vars.border,

	width: 'fit-content',
});
globalStyle(`${buildInfoTable} :is(th, td)`, {
	padding: `${consts.space.xs} ${consts.space.sm}`,
	border: vars.border,
	textAlign: 'unset',
});
globalStyle(`${buildInfoTable} th`, {
	fontWeight: 600,
});
