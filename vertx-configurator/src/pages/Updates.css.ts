import { globalStyle, style } from '@vanilla-extract/css';
import {
	borderBase,
	button,
	fontSize,
	space,
	transition,
	vars,
} from '../index.css';

export const localUpdate = button;
globalStyle(`${localUpdate} > input`, {
	display: 'none',
});

export const updateDialog = style({
	color: vars.colors.fg,
	margin: 'auto',
	padding: space.md,
	background: vars.colors.bgSurface,
	border: vars.border,
	borderRadius: space.md,
	outline: 'none',

	display: 'flex',
	flexDirection: 'column',
	gap: space.sm,
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
	fontSize: fontSize.heading[2],
	fontWeight: 'normal',
});
globalStyle(`${updateDialog} progress`, {
	height: space.md,
	borderRadius: space.md,
	border: `${borderBase} ${vars.colors.green}`,
	background: 'none',
});
globalStyle(`${updateDialog} ::-moz-progress-bar`, {
	background: `oklch(${vars.colors.raw.green} / 0.7)`,
	transition: `width ${transition.short} ${transition.timing}`,
});
export const updateDialogButton = style([
	button,
	{
		alignSelf: 'end',
	},
]);
