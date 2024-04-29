import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { menuClosed } from './MenuButton.css';
import { height as statusBarHeight } from './StatusBar.css';
import { consts, vars } from './index.css';

const padding = consts.size.md;

export const width = createVar('navigation-width');
globalStyle(':root', {
	vars: {
		[width]: `calc(${consts.pagePadding.left} + clamp(200px, 30vw, 300px))`,
	},
	'@media': {
		[consts.isMobile]: {
			vars: {
				// Matches main's padding from App.css.ts
				[width]: `calc(${consts.pagePadding.left} + min(100vw - ${consts.size.lg}, 300px))`,
			},
		},
	},
});

const visibilityDelay = createVar('visibility-delay');
export const root = style({
	position: 'fixed',
	top: statusBarHeight,
	left: 0,
	bottom: 0,
	paddingLeft: consts.pagePadding.left,
	paddingBottom: consts.pagePadding.bottom,
	overflowY: 'auto',
	overscrollBehavior: 'contain',

	background: vars.colors.bgRoot,
	borderRight: vars.border,

	display: 'flex',
	flexDirection: 'column',
	width,

	'@media': {
		[consts.isMobile]: {
			vars: {
				[visibilityDelay]: '0s',
			},

			transition: `transform ${consts.transition.short}, opacity ${consts.transition.short}, visibility 0s linear ${visibilityDelay}`,
			transformOrigin: 'left',
			selectors: {
				[`${menuClosed} ~ &`]: {
					vars: {
						[visibilityDelay]: consts.transition.shortTime,
					},

					transform: 'translateX(-4px) scale(98%)',
					opacity: '0.6',
					visibility: 'hidden',
				},
			},
		},
	},
});

export const nav = style({
	display: 'flex',
	flexDirection: 'column',
	flexGrow: 1,
	padding: padding,
	gap: consts.size.xs,
});
const navLink = `${nav} > a`;
globalStyle(navLink, {
	textDecoration: 'none',
	lineHeight: consts.size.button,
	paddingInline: consts.size.md,
	display: 'flex',
	alignItems: 'center',
	overflow: 'hidden',
});
globalStyle(`${navLink}.active`, {
	background: vars.colors.bgSurface,
	borderColor: vars.colors.border,
});
globalStyle(`${navLink} > :not(:last-child)`, {
	flexShrink: 0,
});
globalStyle(`${navLink} > :last-child`, {
	overflow: 'hidden',
	textOverflow: 'ellipsis',
	whiteSpace: 'nowrap',
});

export const navIcon = style({
	marginInlineEnd: consts.size.sm,
});

export const modelsHeader = style({
	display: 'inline-flex',
	alignItems: 'center',
	justifyContent: 'space-between',

	fontSize: '1.05em',
	fontWeight: 600,
	marginTop: consts.size.sm,
});

export const newModel = style({
	cursor: 'pointer',

	display: 'inline-flex',
	justifyContent: 'center',
	alignItems: 'center',
	width: consts.size.button,
	height: consts.size.button,
	borderRadius: consts.size.sm,
});

globalStyle(`${navLink}, ${newModel}`, {
	color: 'inherit',
	background: 'none',
	borderRadius: consts.size.sm,
	border: `${consts.border.base} transparent`,
	outline: 'none',
});
globalStyle(`:is(${navLink}:not(.active), ${newModel}):hover`, {
	'@media': {
		'(hover: hover)': {
			background: vars.colors.bgHover,
		},
	},
});
globalStyle(`:is(${navLink}, ${newModel}):focus-visible`, {
	borderColor: vars.colors.borderFocus,
});
