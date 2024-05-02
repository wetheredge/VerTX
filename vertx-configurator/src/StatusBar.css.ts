import {
	createVar,
	globalStyle,
	style,
	styleVariants,
} from '@vanilla-extract/css';
import { padding as menuButtonPadding } from './MenuButton.css';
import { ApiStatus } from './api';
import {
	borderWidth,
	fontSize,
	mediaIsMobile,
	pagePadding,
	space,
	vars,
} from './index.css';

const rootPadding = space.sm;
const statusFontSize = fontSize.small;
const statusPadding = space.xs;
export const height = createVar('status-bar-height');
globalStyle(':root', {
	vars: {
		[height]: `calc(${pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${borderWidth}) * 2))`,
	},
	'@media': {
		[mediaIsMobile]: {
			vars: {
				// Add space.button -- height of the menu button
				[height]: `calc(${pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${borderWidth}) * 2, ${space.button}))`,
			},
		},
	},
});

export const root = style({
	position: 'fixed',
	inset: 0,
	bottom: 'auto',
	zIndex: 10,
	maxWidth: '100vw',

	background: vars.colors.bgSurface,
	borderBottom: vars.border,

	display: 'flex',
	alignItems: 'center',
	gap: space.sm,
	paddingTop: `calc(${rootPadding} + ${pagePadding.top})`,
	paddingBottom: rootPadding,
	paddingLeft: `calc(${space.md} + ${pagePadding.left})`,
	paddingRight: `calc(${space.md} + ${pagePadding.right})`,

	'@media': {
		[mediaIsMobile]: {
			paddingLeft: `calc(${space.md} + ${pagePadding.left} - ${menuButtonPadding})`,
		},
	},
});
globalStyle(`${root} > *`, {
	overflow: 'hidden',
});
globalStyle(`${root} > :is(:first-child, :last-child)`, {
	display: 'inline-flex',
	alignItems: 'center',
	flex: '1 1 0',
});
globalStyle(`${root} > :last-child`, {
	justifyContent: 'flex-end',
	gap: space.sm,
	fontFeatureSettings: '"tnum"',
});

export const vertxWithVersion = style({
	'@media': {
		[mediaIsMobile]: {
			display: 'none',
		},
	},
});

export const vertxWithoutVersion = style({
	'@media': {
		[`not ${mediaIsMobile}`]: {
			display: 'none',
		},
	},
});

const apiStatusColor = createVar('api-status-color');
const apiStatusBase = style({
	padding: `${statusPadding} ${space.sm}`,
	borderRadius: space.md,
	fontSize: statusFontSize,
	lineHeight: 1,
	background: `oklch(${apiStatusColor} / 0.7)`,
	border: `${borderWidth} solid oklch(${apiStatusColor})`,
});

export const apiStatus = styleVariants({
	[ApiStatus.Connected]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.green } },
	],
	[ApiStatus.Connecting]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.orange } },
	],
	[ApiStatus.LostConnection]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.red } },
	],
});
