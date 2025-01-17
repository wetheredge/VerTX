const maxWidth = '1200px';
const maxMobileWidth = '600px';
export const mediaIsMobile = `(max-width: ${maxMobileWidth})`;

const pagePaddingBase = `max(0px, calc((100vw - ${maxWidth}) / 2))`;
export const pagePadding = {
	top: 'env(safe-area-inset-top)',
	bottom: 'env(safe-area-inset-bottom)',
	left: `calc(${pagePaddingBase} + env(safe-area-inset-left))`,
	right: `calc(${pagePaddingBase} + env(safe-area-inset-right))`,
} as const;

export const borderWidth = '1px';
export const borderBase = `${borderWidth} solid`;

export const size = {
	xs: '0.25rem',
	sm: '0.5rem',
	md: '1rem',
	lg: '2rem',
	touchTarget: '2.5rem',
	buttonIcon: '1.25rem',
} as const;

export const fontSize = {
	small: '0.875rem',
	normal: '1rem',
	heading: ['1.75rem', '1.5rem', '1.25rem'],
} as const;

const transitionShort = '300ms';
export const transition = {
	shortTime: transitionShort,
	short: `${transitionShort} cubic-bezier(0.61, 1, 0.88, 1)`,
} as const;

export const opacity = 0.7;
export const chroma = 0.17;
export const hues = {
	green: 158,
	blue: 250,
	orange: 52,
	red: 12,
} as const;
