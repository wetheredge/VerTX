import {
	createGlobalTheme,
	createGlobalThemeContract,
	generateIdentifier,
	globalStyle,
} from '@vanilla-extract/css';
import { borderBase, chroma, hues } from './constants.ts';

export const theme = createGlobalThemeContract(
	{
		color: {
			fg: null,
			fgDim: null,
			bgRoot: null,
			bgSurface: null,
			bgHover: null,
			borderSurface: null,

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

const lc = `${theme.color.lightness} ${chroma}`;

createGlobalTheme(':root', theme, {
	color: {
		fg: 'black',
		fgDim: 'black',
		bgRoot: 'oklch(95% 0 0)',
		bgSurface: 'white',
		bgHover: 'oklch(0% 0 0 / 8%)',
		borderSurface: 'oklch(80% 0 0)',

		lightness: '72%',

		green: `oklch(${lc} ${hues.green})`,
		blue: `oklch(${lc} ${hues.blue})`,
		orange: `oklch(${lc} ${hues.orange})`,
		red: `oklch(${lc} ${hues.red})`,
	},
	border: `${borderBase} ${theme.color.borderSurface}`,
});

globalStyle(':root', {
	'@media': {
		'(prefers-color-scheme: dark)': {
			vars: {
				[theme.color.fg]: 'white',
				[theme.color.fgDim]: 'oklch(90% 0 0)',
				[theme.color.bgRoot]: 'black',
				[theme.color.bgSurface]: 'oklch(18% 0 0)',
				[theme.color.bgHover]: 'oklch(100% 0 0 / 11%)',
				[theme.color.borderSurface]: 'oklch(40% 0 0)',

				[theme.color.lightness]: '65%',
			},
		},
	},
});

globalStyle('body', {
	fontFamily: 'system-ui, sans-serif',
	color: theme.color.fg,
	background: theme.color.bgSurface,
});
