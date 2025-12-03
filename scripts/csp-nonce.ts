#!/usr/bin/env node

import { readFile, writeFile } from 'node:fs/promises';
import { argv } from 'node:process';
import { type Element, HTMLRewriter } from 'lol-html';
import { glob } from 'tinyglobby';
import { panic } from '#utils/cli';

const dir = argv[2];
if (!dir) {
	panic('Must pass directory as first argument');
}

const nonce = '{{placeholder "http.request.uuid"}}';

const decoder = new TextDecoder('utf-8', { fatal: true });
function rewrite(raw: Uint8Array): string {
	// Caddy needs double quotes in the template, but HTMLRewriter helpfully escapes them, so give
	// HTMLRewriter a safe string, then replace it again with the proper template.
	const marker = '$$NONCE$$';

	const chunks: Array<Uint8Array> = [];
	const rewriter = new HTMLRewriter('utf8', (chunk: Uint8Array) => {
		chunks.push(chunk);
	});
	rewriter.on('script, style', {
		element(element: Element) {
			element.setAttribute('nonce', marker);
		},
	});

	rewriter.write(raw);
	rewriter.end();

	return decoder.decode(Buffer.concat(chunks)).replaceAll(marker, nonce);
}

const promises: Array<Promise<unknown>> = [];
for (const path of await glob('**.html', { cwd: dir, absolute: true })) {
	if (!path.endsWith('.html')) {
		continue;
	}

	promises.push(
		readFile(path).then((raw) => {
			const final = rewrite(raw);
			return writeFile(path, final);
		}),
	);
}

await Promise.all(promises);
