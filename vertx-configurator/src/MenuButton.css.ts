import { generateIdentifier, globalStyle, style } from '@vanilla-extract/css';
import { consts, vars } from './index.css';

export const id = generateIdentifier('navigation-menu-state');
export const menuOpen = `#${id}:checked`;
export const menuClosed = `#${id}:not(:checked)`;

export const iconSize = '1.25em';
const size = consts.space.button;
export const padding = `calc((${size} - ${iconSize}) / 2)`;

export const button = style({
	cursor: 'pointer',

	display: 'none',
	justifyContent: 'center',
	alignItems: 'center',
	width: consts.space.button,
	height: consts.space.button,
	flex: '0 0 auto',
	outline: 'none',
	borderRadius: consts.space.sm,
	border: `${consts.border.base} transparent`,

	':focus-visible': {
		borderColor: vars.colors.borderFocus,
	},

	'@media': {
		[consts.isMobile]: {
			display: 'inline-flex',
		},

		'(hover: hover)': {
			':hover': {
				background: vars.colors.bgHover,
			},
		},
	},
});

export const iconMenu = generateIdentifier('icon-menu');
export const iconX = generateIdentifier('icon-x');
globalStyle(`.${iconMenu}, .${iconX}`, {
	position: 'absolute',
});
globalStyle(`${menuOpen} ~ * .${iconMenu}, ${menuClosed} ~ * .${iconX}`, {
	display: 'none',
});

export const state = style({
	display: 'none',
});
