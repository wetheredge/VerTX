import {
	type ComplexStyleRule,
	createGlobalTheme,
	createGlobalThemeContract,
	fontFace,
	generateIdentifier,
	globalStyle,
	layer,
	style,
} from '@vanilla-extract/css';
import inter from 'inter-ui/variable/InterVariable.woff2?url';

export const vars = createGlobalThemeContract(
	{
		colors: {
			fg: null,
			fgDim: null,
			bgRoot: null,
			bgSurface: null,
			bgHover: null,
			border: null,
			borderFocus: null,
			borderFocusDanger: null,

			inputBg: null,
			inputBgInvalid: null,

			lightness: null,

			green: null,
			blue: null,
			orange: null,
			red: null,
		},
		border: null,
	},
	(_, path) => generateIdentifier(path.join('-')),
);

const font = fontFace(
	{
		src: `local('InterVariable'), url(${inter}) format("woff2")`,
		fontStyle: 'normal',
		fontWeight: '100 900',
		fontDisplay: 'swap',
		fontFeatureSettings: '"case", "dlig"',
	},
	'inter',
);

const maxWidth = '1200px';
const maxMobileWidth = '600px';
export const mediaIsMobile = `(max-width: ${maxMobileWidth})`;
const pagePaddingBase = `max(0px, calc((100vw - ${maxWidth}) / 2))`;
const pagePadding = {
	top: 'env(safe-area-inset-top)',
	bottom: 'env(safe-area-inset-bottom)',
	left: `calc(${pagePaddingBase} + env(safe-area-inset-left))`,
	right: `calc(${pagePaddingBase} + env(safe-area-inset-right))`,
} as const;

const borderWidth = '1px';
const border = {
	base: `${borderWidth} solid`,
	width: borderWidth,
} as const;

export const buttonIconSize = '1.25rem';
const size = {
	xs: '0.25rem',
	sm: '0.5rem',
	md: '1rem',
	lg: '2rem',
	button: '2.5rem',
	buttonIcon: buttonIconSize,
} as const;

const fontSize = {
	small: '0.875rem',
	normal: '1rem',
	heading: ['1.75rem', '1.5rem', '1.25rem'],
} as const;

const transitionShort = '300ms';
const transition = {
	shortTime: transitionShort,
	short: `${transitionShort} cubic-bezier(0.61, 1, 0.88, 1)`,
} as const;

const colors = {
	lc: `${vars.colors.lightness} 0.15`,
	hues: {
		green: 158,
		blue: 250,
		orange: 52,
		red: 12,
	},
	opacity: 0.7,
} as const;

export const consts = {
	border,
	colors,
	fontSize,
	isMobile: mediaIsMobile,
	pagePadding,
	size,
	transition,
} as const;

createGlobalTheme(':root', vars, {
	colors: {
		fg: 'black',
		fgDim: 'black',
		bgRoot: 'oklch(95% 0 0)',
		bgSurface: 'white',
		bgHover: 'oklch(0% 0 0 / 8%)',
		border: 'oklch(80% 0 0)',
		borderFocus: `oklch(55% 0.25 ${colors.hues.blue})`,
		borderFocusDanger: `oklch(55% 0.25 ${colors.hues.red})`,

		inputBg: 'oklch(98% 0 0)',
		inputBgInvalid: `oklch(94% 0.03 ${colors.hues.red})`,

		lightness: '72%',

		green: `oklch(${colors.lc} ${colors.hues.green})`,
		blue: `oklch(${colors.lc} ${colors.hues.blue})`,
		orange: `oklch(${colors.lc} ${colors.hues.orange})`,
		red: `oklch(${colors.lc} ${colors.hues.red})`,
	},
	border: `${border.base} ${vars.colors.border}`,
});

globalStyle(':root', {
	'@media': {
		'(prefers-color-scheme: dark)': {
			vars: {
				[vars.colors.fg]: 'white',
				[vars.colors.fgDim]: 'oklch(90% 0 0)',
				[vars.colors.bgRoot]: 'black',
				[vars.colors.bgSurface]: 'oklch(19% 0 0)',
				[vars.colors.bgHover]: 'oklch(100% 0 0 / 13%)',
				[vars.colors.border]: 'oklch(40% 0 0)',

				[vars.colors.inputBg]: 'oklch(25% 0 0)',
				[vars.colors.inputBgInvalid]:
					`oklch(36% 0.05 ${colors.hues.red})`,

				[vars.colors.lightness]: '65%',
			},
		},
	},
});

globalStyle('body', {
	fontFamily: `${font}, system-ui, sans-serif`,
	color: vars.colors.fg,
	background: vars.colors.bgSurface,
});

const components = layer('components');

const buttonBase: ComplexStyleRule = {
	cursor: 'pointer',
	outline: 'none',

	borderRadius: size.sm,
	color: vars.colors.fg,

	':focus-visible': {
		borderColor: vars.colors.borderFocus,
	},

	'@media': {
		'(hover: hover)': {
			':hover': {
				background: vars.colors.bgHover,
			},
		},
	},
};

export const button = style({
	'@layer': {
		[components]: {
			...buttonBase,

			width: 'fit-content',
			background: vars.colors.inputBg,
			padding: `${size.xs} ${size.sm}`,
			border: vars.border,
		},
	},
});

export const iconButton = style({
	'@layer': {
		[components]: {
			...buttonBase,

			display: 'inline-flex',
			flex: '0 0 auto',
			justifyContent: 'center',
			alignItems: 'center',
			width: size.button,
			height: size.button,
			border: `${border.base} transparent`,
			background: 'none',
		},
	},
});
