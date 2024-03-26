import { generateIdentifier, globalStyle, style } from '@vanilla-extract/css';
import { borderBase, mediaIsMobile, space, vars } from './index.css';

export const id = generateIdentifier('navigation-menu-state');
export const menuOpen = `#${id}:checked`;
export const menuClosed = `#${id}:not(:checked)`;

export const iconSize = '1.25em';
const size = space.button;
export const padding = `calc((${size} - ${iconSize}) / 2)`;

export const button = style({
	cursor: 'pointer',

	display: 'none',
	justifyContent: 'center',
	alignItems: 'center',
	width: space.button,
	height: space.button,
	flex: '0 0 auto',
	outline: 'none',
	borderRadius: space.sm,
	border: `${borderBase} transparent`,

	':focus-visible': {
		borderColor: vars.colors.borderFocus,
	},

	'@media': {
		[mediaIsMobile]: {
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
