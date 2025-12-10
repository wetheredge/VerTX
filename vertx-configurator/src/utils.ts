export const typedGetElementById = document.getElementById.bind(document) as <
	T extends HTMLElement,
>(
	id: string,
) => T;

export const typedQuerySelector = document.querySelector.bind(document) as <
	T extends HTMLElement,
>(
	id: string,
) => T;
