#!/usr/bin/env bun

import { Glob } from 'bun';
import { panic } from '#utils/cli';

const dir = Bun.argv[2];
if (!dir) {
	panic('Must pass directory as first argument');
}

// Caddy needs double quotes in the template, but HTMLRewriter helpfully escapes them, so give
// HTMLRewriter a safe string, then replace it again with the proper template.
const marker = '$$NONCE$$';

const rewriter = new HTMLRewriter().on('script, style', {
	element(element) {
		element.setAttribute('nonce', marker);
	},
});

const promises: Array<Promise<unknown>> = [];
for await (const path of new Glob('**.html').scan(dir)) {
	const file = Bun.file(`${dir}/${path}`);
	const withMarker = await rewriter.transform(new Response(file)).text();
	const final = withMarker.replaceAll(
		marker,
		'{{placeholder "http.request.uuid"}}',
	);
	promises.push(Bun.write(file, final));
}

await Promise.all(promises);
