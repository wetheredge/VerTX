import { globalStyle, style } from '@vanilla-extract/css';
import { size } from '../styles/constants.ts';
import { theme } from '../styles/theme.css.ts';

export const buildInfoTable = style({
	borderCollapse: 'collapse',
	borderSpacing: 0,
	border: theme.border,

	width: 'fit-content',
});
globalStyle(`${buildInfoTable} :is(th, td)`, {
	padding: `${size.xs} ${size.sm}`,
	border: theme.border,
	textAlign: 'unset',
});
globalStyle(`${buildInfoTable} th`, {
	fontWeight: 600,
});
