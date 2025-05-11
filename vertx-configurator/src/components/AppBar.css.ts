import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { button } from '~/styles/components.css';
import {
	borderWidth,
	fontSize,
	mediaIsMobile,
	pagePadding,
	size,
} from '~/styles/constants.ts';
import { theme } from '~/styles/theme.css';

const rootPadding = size.sm;

export const height = createVar('app-bar-height');
const baseHeight = `${pagePadding.top} + ${rootPadding} * 2`;
const textHeight = '1rem';
const saveHeight = `${size.touchTarget} + ${borderWidth} * 2`;
globalStyle(':root', {
	vars: {
		[height]: `calc(${baseHeight} + max(${textHeight}, ${saveHeight}))`,
	},
	'@media': {
		[mediaIsMobile]: {
			vars: {
				[height]: `calc(${baseHeight} + max(${textHeight}, ${saveHeight}, ${size.touchTarget}))`,
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

	background: theme.color.bgSurface,
	borderBottom: theme.border,

	display: 'flex',
	alignItems: 'center',

	height,
	paddingLeft: `calc(${size.md} + ${pagePadding.left})`,
	paddingRight: `calc(${size.md} + ${pagePadding.right})`,

	'@media': {
		[mediaIsMobile]: {
			paddingLeft: `calc(${size.md} + ${pagePadding.left} - ((${size.touchTarget} - ${size.buttonIcon}) / 2))`,
		},
	},
});

export const title = style({
	fontSize: fontSize.title,
});

export const save = style([
	button,
	{
		display: 'inline-block',
		marginInlineStart: 'auto',
	},
]);
