import { createVar, layer, style } from '@vanilla-extract/css';
import { borderBase, size } from './constants.ts';
import { theme } from './theme.css.ts';

const components = layer('components');

const inputColor = createVar('input-color');
const inputBgTransparency = createVar('input-bg-transparency');
const inputBorderTransparency = createVar('input-border-transparency');
export const inputBase = style({
	'@layer': {
		[components]: {
			vars: {
				[inputColor]: 'oklch(53% 0 0)',
				[inputBgTransparency]: '81%',
				[inputBorderTransparency]: '35%',
			},

			selectors: {
				'&.primary': { vars: { [inputColor]: theme.color.blue } },
			},

			outline: 'none',
			borderRadius: size.sm,

			background: `color-mix(in srgb, ${inputColor}, transparent ${inputBgTransparency})`,
			border: `${borderBase} color-mix(in srgb, ${inputColor}, transparent ${inputBorderTransparency})`,

			':focus-visible': {
				vars: {
					[inputBgTransparency]: '65%',
					[inputBorderTransparency]: '0%',
				},
			},

			':invalid': { vars: { [inputColor]: theme.color.red } },
		},
	},
});

export const button = style([
	inputBase,
	{
		'@layer': {
			[components]: {
				cursor: 'pointer',
				color: theme.color.fg,

				lineHeight: 1,
				width: 'fit-content',
				minWidth: size.touchTarget,
				minHeight: size.touchTarget,
				padding: `${size.sm} ${size.md}`,

				'@media': {
					'(hover: hover)': {
						':hover': { vars: { [inputBgTransparency]: '73%' } },
					},
				},
			},
		},
	},
]);

export const iconButton = style({
	'@layer': {
		[components]: {
			cursor: 'pointer',
			outline: 'none',

			display: 'inline-flex',
			flex: '0 0 auto',
			justifyContent: 'center',
			alignItems: 'center',
			width: size.touchTarget,
			height: size.touchTarget,

			background: 'none',
			color: theme.color.fg,
			border: `${borderBase} transparent`,
			borderRadius: size.sm,

			':focus-visible': {
				borderColor: theme.color.blue,
			},

			'@media': {
				'(hover: hover)': {
					':hover': {
						background: theme.color.bgHover,
					},
				},
			},
		},
	},
});
