import { globalStyle } from '@vanilla-extract/css';
import { height as appBarHeight } from '~/components/AppBar.css.ts';
import { width as navWidth } from '~/components/Navigation.css.ts';
import { menuOpen } from '~/components/menu-selectors.ts';
import {
	fontSize,
	mediaIsMobile,
	pagePadding,
	size,
	transition,
} from '~/styles/constants.ts';
import { theme } from '~/styles/theme.css.ts';

globalStyle('#root', {
	overflowX: 'hidden',
});

globalStyle('main', {
	position: 'relative',
	minHeight: `calc(100dvh - ${appBarHeight})`,
	padding: size.lg,
	paddingRight: `calc(${size.lg} + ${pagePadding.right})`,
	paddingBottom: `calc(${size.lg} + ${pagePadding.bottom})`,
	marginTop: appBarHeight,
	marginLeft: navWidth,

	background: theme.color.bgSurface,

	display: 'flex',
	flexDirection: 'column',
	gap: size.md,

	'@media': {
		[mediaIsMobile]: {
			padding: size.md,
			width: '100vw',
			marginLeft: 0,
			transition: `margin-left ${transition.short}, filter ${transition.short}`,
		},
	},
});

globalStyle(`${menuOpen} ~ main`, {
	'@media': {
		[mediaIsMobile]: {
			marginLeft: navWidth,
			filter: 'saturate(0.4)',
			pointerEvents: 'none',
		},
	},
});
globalStyle(`body:has(#root > ${menuOpen})`, {
	overflowY: 'hidden',
});

globalStyle('main :is(h1, h2, h3, p)', {
	margin: 0,
});
globalStyle('main :is(h1, h2, h3)', {
	fontWeight: 600,
});
globalStyle('main > :not(h1) ~ h2', {
	marginTop: size.md,
});
globalStyle('main h1', {
	fontSize: fontSize.heading[0],
});
globalStyle('main h2', {
	fontSize: fontSize.heading[1],
});
globalStyle('main h3', {
	fontSize: fontSize.heading[2],
});
