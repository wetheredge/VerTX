import { globalStyle, style } from '@vanilla-extract/css';
import { button, consts, vars } from '../index.css';

export const localUpdate = button;
globalStyle(`${localUpdate} > input`, {
	display: 'none',
});

export const updateDialog = style({
	color: vars.colors.fg,
	margin: 'auto',
	padding: consts.space.md,
	background: vars.colors.bgSurface,
	border: vars.border,
	borderRadius: consts.space.md,
	outline: 'none',

	display: 'flex',
	flexDirection: 'column',
	gap: consts.space.sm,
	width: '30ch',

	':focus-visible': {
		borderColor: vars.colors.borderFocus,
	},

	'::backdrop': {
		backdropFilter: 'blur(3px) brightness(0.6) saturate(0.8)',
	},

	selectors: {
		'&:not([open])': {
			display: 'none',
		},
	},
});
globalStyle(`${updateDialog} h2`, {
	fontSize: consts.fontSize.heading[2],
	fontWeight: 'normal',
});
globalStyle(`${updateDialog} progress`, {
	height: consts.space.md,
	borderRadius: consts.space.md,
	border: `${consts.border.base} ${vars.colors.green}`,
	background: 'none',
});
globalStyle(`${updateDialog} ::-moz-progress-bar`, {
	background: `oklch(${vars.colors.raw.green} / 0.7)`,
	transition: `width ${consts.transition.short}`,
});
export const updateDialogButton = style([
	button,
	{
		alignSelf: 'end',
	},
]);
