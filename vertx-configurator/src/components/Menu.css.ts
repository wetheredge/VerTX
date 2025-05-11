import { globalStyle } from '@vanilla-extract/css';
import { mediaIsMobile } from '~/styles/constants.ts';
import { menuClosed, menuOpen, stateId } from './menu-selectors.ts';

const menuButton = `[for="${stateId}"]`;

globalStyle(menuButton, {
	'@media': {
		[`not ${mediaIsMobile}`]: {
			display: 'none',
		},
	},
});

globalStyle(
	`${menuOpen} ~ * ${menuButton} > :first-child, ${menuClosed} ~ * ${menuButton} > :last-child`,
	{
		display: 'none',
	},
);
