import { style } from '@vanilla-extract/css';
import {
	borderBase,
	borderWidth,
	chroma,
	fontSize,
	hues,
	opacity,
	size,
} from '../../styles/constants.ts';
import { theme } from '../../styles/theme.css.ts';

export { iconButton } from '../../styles/components.css.ts';

const channelHeight = size.md;
const centerTick = size.xs;
const labelWidth = `calc(7ch + ${size.sm})`;
export const channel = style({
	vars: {
		'--bar1': theme.color.blue,
		'--bar2': `oklch(${theme.color.lightness} ${chroma} ${hues.blue} / ${opacity})`,
	},

	position: 'relative',
	display: 'inline-block',
	height: channelHeight,
	borderRadius: channelHeight,
	border: `${borderBase} ${theme.color.blue}`,
	backgroundImage:
		'linear-gradient(to right, var(--bar1) 0%, var(--bar2) var(--value), transparent 0% 100%)',
	backgroundOrigin: 'border-box',

	fontFeatureSettings: '"tnum"',
	fontSize: fontSize.small,
	marginRight: labelWidth,

	'::before': {
		content: '',
		position: 'absolute',
		display: 'inherit',
		height: `calc(${channelHeight} + 2 * ${centerTick})`,
		width: borderWidth,
		inset: `calc(-${centerTick} - ${borderWidth}) calc(50% - ${borderWidth} / 2)`,
		borderBlock: `${centerTick} solid`,
		borderColor: 'inherit',
	},

	'::after': {
		content: 'attr(aria-valuenow) "%"',
		position: 'absolute',
		left: '100%',
		right: 0,
		width: labelWidth,
		textAlign: 'right',
		lineHeight: '100%',
	},

	selectors: {
		'&.center': {
			vars: { '--min': 'min(50%, var(--value))' },
			backgroundImage:
				'linear-gradient(to right, transparent 0% var(--min), var(--bar2) var(--min), var(--bar1) 50%, var(--bar2) max(50%, var(--value)), transparent 0% 100%)',
		},
		'&.center::before': {
			background: theme.color.blue,
		},
	},
});
