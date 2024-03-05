import { style, styleVariants } from '@vanilla-extract/css';
import { borderRadius, space, vars } from '../index.css';
import { ApiStatus } from '../lib/api';

export const root = style({
	display: 'grid',
	gridTemplateColumns: '1fr auto 1fr',
	alignItems: 'baseline',
	padding: space.sm,
	gap: space.sm,
	justifyContent: 'center',

	position: 'sticky',
	top: 0,
});

export const right = style({
	display: 'inline-flex',
	gap: space.sm,
	justifyContent: 'flex-end',
});

const apiStatusBase = style({
	paddingInline: space.sm,
	paddingBlock: space.xs,
	marginBlock: `-${space.xs}`,

	borderRadius: borderRadius.small,
});

export const apiStatus = styleVariants({
	[ApiStatus.Connected]: [
		apiStatusBase,
		{
			background: vars.colors.green,
		},
	],
	[ApiStatus.Connecting]: [
		apiStatusBase,
		{
			background: vars.colors.orange,
		},
	],
	[ApiStatus.LostConnection]: [
		apiStatusBase,
		{
			background: vars.colors.red,
		},
	],
});
