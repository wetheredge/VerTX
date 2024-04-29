import {
	createVar,
	globalStyle,
	style,
	styleVariants,
} from '@vanilla-extract/css';
import { padding as menuButtonPadding } from './MenuButton.css';
import { ApiStatus } from './api';
import { consts, vars } from './index.css';

const rootPadding = consts.space.sm;
const statusFontSize = consts.fontSize.small;
const statusPadding = consts.space.xs;
export const height = createVar('status-bar-height');
globalStyle(':root', {
	vars: {
		[height]: `calc(${consts.pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${consts.border.width}) * 2))`,
	},
	'@media': {
		[consts.isMobile]: {
			vars: {
				// Add space.button -- height of the menu button
				[height]: `calc(${consts.pagePadding.top} + ${rootPadding} * 2 + max(1rem, ${statusFontSize} + (${statusPadding} + ${consts.border.width}) * 2, ${consts.space.button}))`,
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
	gap: consts.space.sm,
	paddingTop: `calc(${rootPadding} + ${consts.pagePadding.top})`,
	paddingBottom: rootPadding,
	paddingLeft: `calc(${consts.space.md} + ${consts.pagePadding.left})`,
	paddingRight: `calc(${consts.space.md} + ${consts.pagePadding.right})`,

	'@media': {
		[consts.isMobile]: {
			paddingLeft: `calc(${consts.space.md} + ${consts.pagePadding.left} - ${menuButtonPadding})`,
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
	gap: consts.space.sm,
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

const apiStatusColor = createVar('api-status-color');
const apiStatusBase = style({
	padding: `${statusPadding} ${consts.space.sm}`,
	borderRadius: consts.space.md,
	fontSize: statusFontSize,
	lineHeight: 1,
	background: `oklch(${apiStatusColor} / 0.7)`,
	border: `${consts.border.width} solid oklch(${apiStatusColor})`,
});

const partialApiStatus = styleVariants({
	[ApiStatus.Connecting]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.orange } },
	],
	[ApiStatus.Connected]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.green } },
	],
	[ApiStatus.LostConnection]: [
		apiStatusBase,
		{ vars: { [apiStatusColor]: vars.colors.raw.red } },
	],
});
export const apiStatus: Record<ApiStatus, string> = {
	...partialApiStatus,
	[ApiStatus.Reconnecting]: partialApiStatus[ApiStatus.Connecting],
};
