/* @refresh reload */
import { render } from 'solid-js/web';
import 'modern-normalize';

import './index.css.ts';
import App from '~/components/App';

const root = document.getElementById('root');

render(() => <App />, root!);
