/* @refresh reload */
import { render } from 'solid-js/web';

import 'modern-normalize';
import { App } from './components/App';
import './index.css.ts';

// biome-ignore lint/style/noNonNullAssertion: #root is in the index.html template
const root = document.getElementById('root')!;

render(() => <App />, root);
