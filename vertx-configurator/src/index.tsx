/* @refresh reload */
import { render } from 'solid-js/web';

import { Navigate, Route, Router } from '@solidjs/router';
import { type Component, lazy } from 'solid-js';
import { App } from './App';
import './index.css.ts';
import './reset.css';

// biome-ignore lint/style/noNonNullAssertion: #root is in the index.html template
const root = document.getElementById('root')!;

const About = lazy(() => import('./pages/About.tsx'));
const Settings = lazy(() => import('./pages/Settings.tsx'));
const Updates = lazy(() => import('./pages/Updates.tsx'));
const Hardware = lazy(() => import('./pages/Hardware.tsx'));
const Model = lazy(
	() => import('./pages/Model.tsx') as Promise<{ default: Component }>,
);
const NotFound = () => <Navigate href="/" />;

render(
	() => (
		<Router base={import.meta.env.BASE_URL} root={App}>
			<Route path="/settings" component={Settings} />
			<Route path="/updates" component={Updates} />
			<Route path="/hardware" component={Hardware} />
			<Route path="/model/:id" component={Model} />
			<Route path="/" component={About} />
			<Route path="*" component={NotFound} />
		</Router>
	),
	root,
);
