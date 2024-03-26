import { globalStyle } from '@vanilla-extract/css';
import { menuOpen } from './MenuButton.css';
import { width as navWidth } from './Navigation.css';
import { height as statusHeight } from './StatusBar.css';
import {
	mediaIsMobile,
	pagePadding,
	space,
	transition,
	vars,
} from './index.css';

globalStyle('#root', {
	overflowX: 'hidden',
});

globalStyle('main', {
	position: 'relative',
	minHeight: `calc(100dvh - ${statusHeight})`,
	padding: space.lg,
	paddingRight: `calc(${space.lg} + ${pagePadding.right})`,
	paddingBottom: `calc(${space.lg} + ${pagePadding.bottom})`,
	background: vars.colors.bgSurface,
	marginTop: statusHeight,
	marginLeft: navWidth,

	display: 'flex',
	flexDirection: 'column',
	gap: space.md,

	'@media': {
		[mediaIsMobile]: {
			padding: space.md,
			width: '100vw',
			marginLeft: 0,
			transition: `margin-left ${transition.short} ${transition.timing}, filter ${transition.short} ${transition.timing}`,
		},
	},
});
globalStyle(`${menuOpen} ~ main`, {
	'@media': {
		[mediaIsMobile]: {
			marginLeft: navWidth,
			filter: 'saturate(0.4) opacity(0.8)',
			pointerEvents: 'none',
		},
	},
});
globalStyle(`body:has(#root > ${menuOpen})`, {
	overflowY: 'hidden',
});
