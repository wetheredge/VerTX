import { globalStyle, style } from '@vanilla-extract/css';

globalStyle('#template', {
	display: 'none',
});

export const menu = style({
	position: 'absolute',

	listStyle: 'none',
});

export const model = style({
	display: 'flex',
	alignItems: 'center',
	justifyContent: 'space-between',
});

globalStyle(`${model} .menu:disabled`, {
	pointerEvents: 'none',
});
