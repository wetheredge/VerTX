import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
	plugins: [solid(), tsconfigPaths()],
	define: {
		'import.meta.env.CODESPACE_NAME': JSON.stringify(
			process.env.CODESPACE_NAME,
		),
	},
});
