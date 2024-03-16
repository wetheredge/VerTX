import { createGlobalTheme } from '@vanilla-extract/css';

export const space = {
	xs: '0.25em',
	sm: '0.5em',
	md: '1em',
	lg: '2em',
};

export const borderRadius = {
	small: space.md,
	normal: space.lg,
};

export const fontSize = {
	small: '0.8em',
	normal: '1em',
};

const color = (hue: number) => `oklch(70% 0.15 ${hue})`;

export const vars = createGlobalTheme(':root', {
	colors: {
		green: color(158),
		blue: color(250),
		orange: color(52),
		red: color(18),
	},
});

// globalStyle(':root', {
// 	'@media': {
// 		'(prefers-color-scheme: dark)': {
// 			vars: {
// 				[vars.colors.green]: 'green',
// 			},
// 		},
// 	},
// });
