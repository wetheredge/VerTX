import { type JSX, onCleanup, onMount } from 'solid-js';
import './App.css';
import { MenuState, closeMenu, isOpen as mobileNavIsOpen } from './MenuButton';
import { Navigation } from './Navigation';
import { StatusBar } from './StatusBar';
import { RequestKind, init as initApi, request } from './api';

export function App(props: { children?: JSX.Element }) {
	initApi();
	request({ kind: RequestKind.BuildInfo });
	request({ kind: RequestKind.GetConfig });

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
