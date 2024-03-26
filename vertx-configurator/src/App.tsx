import { type JSX, createSignal, onCleanup, onMount } from 'solid-js';
import './App.css';
import { MenuState, closeMenu, isOpen as mobileNavIsOpen } from './MenuButton';
import { Navigation } from './Navigation';
import { StatusBar } from './StatusBar';
import createApi, {
	type ResponsePayload,
	RequestKind,
	ResponseKind,
} from './api';
import { unreachable } from './utils';

const DEV_API_PORT = 8080;
const API_HOST =
	import.meta.env.MODE === 'production'
		? location.host
		: import.meta.env.CODESPACE_NAME
			? `${import.meta.env.CODESPACE_NAME}-${DEV_API_PORT}.app.github.dev`
			: `localhost:${DEV_API_PORT}`;

export function App(props: { children?: JSX.Element }) {
	const [build, setBuild] =
		createSignal<ResponsePayload<ResponseKind.BuildInfo>>();
	const [status, setStatus] =
		createSignal<ResponsePayload<ResponseKind.Status>>();

	const api = createApi(API_HOST, ({ kind, payload }) => {
		switch (kind) {
			case ResponseKind.ProtocolVersion:
				console.debug('ProtocolVersion', payload);
				break;
			case ResponseKind.BuildInfo:
				setBuild(payload);
				break;
			case ResponseKind.Status:
				setStatus(payload);
				break;

			default:
				unreachable(kind);
		}
	});

	api.request({ kind: RequestKind.BuildInfo });

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
			<StatusBar
				build={build()}
				status={status()}
				apiStatus={api.status()}
			/>
			<Navigation />
			<main inert={mobileNavIsOpen()} ref={main}>
				{props.children}
			</main>
		</>
	);
}
