import { globalStyle, style } from '@vanilla-extract/css';
import { space, vars } from '../index.css';

export const buildInfoTable = style({
	borderCollapse: 'collapse',
	borderSpacing: 0,
	border: vars.border,

	width: 'fit-content',
});
globalStyle(`${buildInfoTable} :is(th, td)`, {
	padding: `${space.xs} ${space.sm}`,
	border: vars.border,
	textAlign: 'unset',
});
globalStyle(`${buildInfoTable} th`, {
	fontWeight: 600,
});
