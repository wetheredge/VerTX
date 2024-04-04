import { type JSX, onCleanup, onMount } from 'solid-js';
import './App.css';
import { MenuState, closeMenu, isOpen as mobileNavIsOpen } from './MenuButton';
import { Navigation } from './Navigation';
import { StatusBar } from './StatusBar';
import { RequestKind, initApi, request } from './api';

const DEV_API_PORT = 8080;
const API_HOST =
	import.meta.env.MODE === 'production'
		? location.host
		: import.meta.env.CODESPACE_NAME
			? `${import.meta.env.CODESPACE_NAME}-${DEV_API_PORT}.app.github.dev`
			: `localhost:${DEV_API_PORT}`;

export function App(props: { children?: JSX.Element }) {
	initApi(API_HOST);
	request({ kind: RequestKind.BuildInfo });

	let main!: HTMLElement;
	let container: HTMLElement | null;
	const handler = ({ target }: MouseEvent) => {
		if (target === container) {
			closeMenu();
		}
	};
	onMount(() => {
		container = main.parentElement;
		container?.addEventListener('click', handler);
	});
	onCleanup(() => container?.removeEventListener('click', handler));

	return (
		<>
			<MenuState />
			<StatusBar />
			<Navigation />
			<main inert={mobileNavIsOpen()} ref={main}>
				{props.children}
			</main>
		</>
	);
}
