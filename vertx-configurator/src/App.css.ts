import { globalStyle } from '@vanilla-extract/css';
import { menuOpen } from './MenuButton.css';
import { width as navWidth } from './Navigation.css';
import { height as statusHeight } from './StatusBar.css';
import { consts, vars } from './index.css';

globalStyle('#root', {
	overflowX: 'hidden',
});

globalStyle('main', {
	position: 'relative',
	minHeight: `calc(100dvh - ${statusHeight})`,
	padding: consts.size.lg,
	paddingRight: `calc(${consts.size.lg} + ${consts.pagePadding.right})`,
	paddingBottom: `calc(${consts.size.lg} + ${consts.pagePadding.bottom})`,
	background: vars.colors.bgSurface,
	marginTop: statusHeight,
	marginLeft: navWidth,

	display: 'flex',
	flexDirection: 'column',
	gap: consts.size.md,

	'@media': {
		[consts.isMobile]: {
			padding: consts.size.md,
			width: '100vw',
			marginLeft: 0,
			transition: `margin-left ${consts.transition.short}, filter ${consts.transition.short}`,
		},
	},
});
globalStyle(`${menuOpen} ~ main`, {
	'@media': {
		[consts.isMobile]: {
			marginLeft: navWidth,
			filter: 'saturate(0.4) opacity(0.8)',
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
globalStyle('main > :not(h1) ~ h2', {
	marginTop: consts.size.md,
});
globalStyle('main h1', {
	fontSize: consts.fontSize.heading[0],
});
globalStyle('main h2', {
	fontSize: consts.fontSize.heading[1],
});
globalStyle('main h3', {
	fontSize: consts.fontSize.heading[2],
});
