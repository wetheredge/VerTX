import { globalStyle, style } from '@vanilla-extract/css';
import { button, consts, vars } from '../index.css';

export { iconButton } from '../index.css';

export const inputs = style({
	display: 'grid',
	gridTemplateColumns: 'min-content auto min-content',
	gap: consts.size.sm,

	'@media': {
		[`not ${consts.isMobile}`]: {
			columnGap: consts.size.md,
		},
	},
});

globalStyle(`${inputs} > *`, {
	marginBlock: 'auto',
});

const channelHeight = consts.size.md;
const centerTick = consts.size.xs;
const labelWidth = `calc(7ch + ${consts.size.sm})`;
export const analogChannel = style({
	vars: {
		'--bar1': vars.colors.blue,
		'--bar2': `oklch(${consts.colors.lc} ${consts.colors.hues.blue} / ${consts.colors.opacity})`,
	},

	position: 'relative',
	display: 'inline-block',
	height: channelHeight,
	borderRadius: channelHeight,
	border: `${consts.border.base} ${vars.colors.blue}`,
	backgroundImage:
		'linear-gradient(to right, var(--bar1) 0%, var(--bar2) var(--value), transparent 0% 100%)',
	backgroundOrigin: 'border-box',

	fontFeatureSettings: '"tnum"',
	fontSize: consts.fontSize.small,
	marginRight: labelWidth,

	'::before': {
		content: '',
		position: 'absolute',
		display: 'inherit',
		height: `calc(${channelHeight} + 2 * ${centerTick})`,
		width: consts.border.width,
		inset: `calc(-${centerTick} - ${consts.border.width}) calc(50% - ${consts.border.width} / 2)`,
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
			background: vars.colors.blue,
		},
	},
});

export const calibrate = button;
