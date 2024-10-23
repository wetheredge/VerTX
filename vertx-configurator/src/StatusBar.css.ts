import {
	createVar,
	globalStyle,
	style,
	styleVariants,
} from '@vanilla-extract/css';
import { ApiStatus } from './api';
import { consts, vars } from './index.css';

const rootPadding = consts.size.sm;
const statusFontSize = consts.fontSize.small;
const statusPadding = consts.size.xs;
export const height = createVar('status-bar-height');
globalStyle(':root', {
	vars: {
		[height]: `calc(${consts.pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${consts.border.width}) * 2))`,
	},
	'@media': {
		[consts.isMobile]: {
			vars: {
				// Add space.button -- height of the menu button
				[height]: `calc(${consts.pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${consts.border.width}) * 2, ${consts.size.button}))`,
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
	gap: consts.size.sm,
	paddingTop: `calc(${rootPadding} + ${consts.pagePadding.top})`,
	paddingBottom: rootPadding,
	paddingLeft: `calc(${consts.size.md} + ${consts.pagePadding.left})`,
	paddingRight: `calc(${consts.size.md} + ${consts.pagePadding.right})`,

	'@media': {
		[consts.isMobile]: {
			paddingLeft: `calc(${consts.size.md} + ${consts.pagePadding.left} - ((${consts.size.button} - ${consts.size.buttonIcon}) / 2))`,
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
	gap: consts.size.sm,
	fontFeatureSettings: '"tnum"',
});

export const vertxWithVersion = style({
	'@media': {
		[consts.isMobile]: {
			display: 'none',
		},
	},
});

export const vertxWithoutVersion = style({
	'@media': {
		[`not ${consts.isMobile}`]: {
			display: 'none',
		},
	},
});

const apiStatusHue = createVar('api-status-hue');
const apiStatusBase = style({
	padding: `${statusPadding} ${consts.size.sm}`,
	borderRadius: consts.size.md,
	fontSize: statusFontSize,
	lineHeight: 1,
	background: `oklch(${consts.colors.lc} ${apiStatusHue} / ${consts.colors.opacity})`,
	border: `${consts.border.base} oklch(${consts.colors.lc} ${apiStatusHue})`,
});

export const apiStatus = styleVariants({
	[ApiStatus.Connected]: [
		apiStatusBase,
		{ vars: { [apiStatusHue]: consts.colors.hues.green.toString() } },
	],
	[ApiStatus.Connecting]: [
		apiStatusBase,
		{ vars: { [apiStatusHue]: consts.colors.hues.orange.toString() } },
	],
	[ApiStatus.NotConnected]: [
		apiStatusBase,
		{ vars: { [apiStatusHue]: consts.colors.hues.red.toString() } },
	],
});
