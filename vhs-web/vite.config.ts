import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import tsconfigPaths from 'vite-tsconfig-paths';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';

export default defineConfig({
	plugins: [solid(), tsconfigPaths(), vanillaExtractPlugin()],
	define: {
		'import.meta.env.CODESPACE_NAME': JSON.stringify(
			process.env.CODESPACE_NAME,
		),
	},
});
