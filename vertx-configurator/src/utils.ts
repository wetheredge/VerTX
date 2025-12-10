export const getElementById = document.getElementById as <
	T extends HTMLElement,
>(
	id: string,
) => T;

export const querySelector = document.querySelector as <T extends HTMLElement>(
	id: string,
) => T;
