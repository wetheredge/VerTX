import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { menuClosed } from './MenuButton.css';
import { height as statusBarHeight } from './StatusBar.css';
import {
	borderBase,
	mediaIsMobile,
	pagePadding,
	space,
	transition,
	vars,
} from './index.css';

const padding = space.md;

export const width = createVar('navigation-width');
globalStyle(':root', {
	vars: {
		[width]: `calc(${pagePadding.left} + clamp(200px, 30vw, 300px))`,
	},
	'@media': {
		[mediaIsMobile]: {
			vars: {
				// Matches main's padding from App.css.ts
				[width]: `calc(${pagePadding.left} + min(100vw - ${space.lg}, 300px))`,
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
	paddingLeft: pagePadding.left,
	paddingBottom: pagePadding.bottom,
	overflowY: 'auto',
	overscrollBehavior: 'contain',

	background: vars.colors.bgRoot,
	borderRight: vars.border,

	display: 'flex',
	flexDirection: 'column',
	width,

	'@media': {
		[mediaIsMobile]: {
			vars: {
				[visibilityDelay]: '0s',
			},

			transition: `transform ${transition.short} ${transition.timing}, opacity ${transition.short} ${transition.timing}, visibility 0s linear ${visibilityDelay}`,
			transformOrigin: 'left',
			selectors: {
				[`${menuClosed} ~ &`]: {
					vars: {
						[visibilityDelay]: transition.short,
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
	gap: space.xs,
});
const navLink = `${nav} > a`;
globalStyle(navLink, {
	textDecoration: 'none',
	lineHeight: space.button,
	paddingInline: space.md,
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
	marginInlineEnd: space.sm,
});

export const modelsHeader = style({
	display: 'inline-flex',
	alignItems: 'center',
	justifyContent: 'space-between',

	fontSize: '1.05em',
	fontWeight: 600,
	marginTop: space.sm,
});

export const newModel = style({
	cursor: 'pointer',

	display: 'inline-flex',
	justifyContent: 'center',
	alignItems: 'center',
	width: space.button,
	height: space.button,
	borderRadius: space.sm,
});

globalStyle(`${navLink}, ${newModel}`, {
	color: 'inherit',
	background: 'none',
	borderRadius: space.sm,
	border: `${borderBase} transparent`,
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
