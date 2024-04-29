import { generateIdentifier, globalStyle, style } from '@vanilla-extract/css';
import { consts, vars } from '../index.css';

export const dangerId = generateIdentifier('danger-zone');
const dangerSelector = `#${dangerId} ~`;
const gap = consts.space.sm;

export const setting = style({
	display: 'flex',
	flexDirection: 'column',
	gap,
});
globalStyle(`${setting} > :is(input, select)`, {
	width: '25ch',

	border: vars.border,
	borderRadius: consts.space.sm,
	color: 'inherit',
	background: vars.colors.bgInput,
	padding: `${consts.space.xs} ${consts.space.sm}`,
	outline: 'none',

	'@media': {
		[consts.isMobile]: {
			width: '100%',
		},
	},
});
globalStyle(`${setting} > :is(input, select):focus-visible`, {
	borderColor: vars.colors.borderFocus,
});
globalStyle(`${dangerSelector} ${setting} > :is(input, select):focus-visible`, {
	borderColor: vars.colors.borderFocusDanger,
});

export const settingCheckbox = style({
	display: 'grid',
	gridTemplateColumns: 'min-content auto',
	gap,
});
globalStyle(`${settingCheckbox} > input`, {
	accentColor: vars.colors.blue,
});
globalStyle(`${dangerSelector} ${settingCheckbox} > input`, {
	accentColor: vars.colors.red,
});
globalStyle(`${settingCheckbox} > span`, {
	gridColumn: 2,
});

const allSettings = `:is(${setting}, ${settingCheckbox})`;

globalStyle(`${allSettings} > label`, {
	fontWeight: 500,
});
globalStyle(`${allSettings} > span`, {
	fontSize: consts.fontSize.small,
	color: vars.colors.fgDim,
});
