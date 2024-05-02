import { generateIdentifier, globalStyle, style } from '@vanilla-extract/css';
import { consts, iconButton } from './index.css';

export const id = generateIdentifier('navigation-menu-state');
export const menuOpen = `#${id}:checked`;
export const menuClosed = `#${id}:not(:checked)`;

export const button = style([
	iconButton,
	{
		'@media': {
			[`not ${consts.isMobile}`]: {
				display: 'none',
			},
		},
	},
]);

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
