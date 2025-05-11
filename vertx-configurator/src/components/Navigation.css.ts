import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { iconButton } from '~/styles/components.css.ts';
import {
	borderBase,
	mediaIsMobile,
	pagePadding,
	size,
	transition,
} from '~/styles/constants.ts';
import { theme } from '~/styles/theme.css.ts';
import { height as appBarHeight } from './AppBar.css.ts';
import { menuClosed } from './menu-selectors.ts';

const padding = size.md;

export const width = createVar('navigation-width');
globalStyle(':root', {
	vars: {
		[width]: `calc(${pagePadding.left} + clamp(200px, 30vw, 300px))`,
	},
	'@media': {
		[mediaIsMobile]: {
			vars: {
				[width]: `calc(${pagePadding.left} + min(100vw - ${size.lg}, 300px))`,
			},
		},
	},
});

const visibilityDelay = createVar('visibility-delay');
export const root = style({
	position: 'fixed',
	top: appBarHeight,
	left: 0,
	bottom: 0,
	paddingLeft: pagePadding.left,
	paddingBottom: pagePadding.bottom,
	overflowY: 'auto',
	overscrollBehavior: 'contain',

	background: theme.color.bgRoot,
	borderRight: theme.border,

	display: 'flex',
	flexDirection: 'column',
	width,

	'@media': {
		[mediaIsMobile]: {
			vars: {
				[visibilityDelay]: '0s',
			},

			transition: `transform ${transition.short}, opacity ${transition.short}, visibility 0s linear ${visibilityDelay}`,
			transformOrigin: 'left',
			selectors: {
				[`${menuClosed} ~ &`]: {
					vars: {
						[visibilityDelay]: transition.shortTime,
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
	padding,
	gap: size.xs,
});

const navLink = `${nav} > a`;
globalStyle(navLink, {
	textDecoration: 'none',
	lineHeight: size.touchTarget,
	paddingInline: size.md,
	display: 'flex',
	alignItems: 'center',
	overflow: 'hidden',
});
globalStyle(`${navLink}.active`, {
	background: theme.color.bgSurface,
	borderColor: theme.color.borderSurface,
});

// NavLink icon
globalStyle(`${navLink} > :first-child`, {
	flexShrink: 0,
	marginInlineEnd: size.sm,
});

// NavLink label
globalStyle(`${navLink} > :last-child`, {
	overflow: 'hidden',
	textOverflow: 'ellipsis',
	whiteSpace: 'nowrap',
});

export const modelsHeader = style({
	display: 'inline-flex',
	alignItems: 'center',
	justifyContent: 'space-between',

	fontSize: '1.05em',
	fontWeight: 600,
	marginTop: size.sm,
});

globalStyle(navLink, {
	color: 'inherit',
	background: 'none',
	borderRadius: size.sm,
	border: `${borderBase} transparent`,
	outline: 'none',
});
globalStyle(`${navLink}:not(.active):hover`, {
	'@media': {
		'(hover: hover)': {
			background: theme.color.bgHover,
		},
	},
});
globalStyle(`${navLink}:focus-visible`, {
	borderColor: theme.color.blue,
});

export const powerButtonContainer = style({
	display: 'inline-flex',
	justifyContent: 'center',
	gap: size.sm,

	position: 'sticky',
	bottom: size.sm,

	borderTop: theme.border,
	margin: padding,
	marginTop: size.sm,
	paddingTop: size.sm,
});

export const powerButton = iconButton;
