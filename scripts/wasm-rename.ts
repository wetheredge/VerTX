#!/usr/bin/env bun

const stdin = await Bun.stdin.text();
const renames = new Map();
for (const line of stdin.split('\n')) {
	const [from, to] = line.split(' => ');
	if (to && from !== to) {
		renames.set(from, to);
	}
}

const base = 'vertx';

const dts = Bun.file(`${base}_bg.wasm.d.ts`);
let dtsContents = await dts.text();
for (const [from, to] of renames) {
	dtsContents = dtsContents.replaceAll(from, to);
}
await Bun.write(dts, dtsContents);

const js = Bun.file(`${base}.js`);
let jsContents = await js.text();
jsContents = jsContents.replaceAll('imports.wbg', 'imports.a');
for (const [from, to] of renames) {
	jsContents = jsContents.replaceAll(`.${from}`, `.${to}`);
	jsContents = jsContents.replaceAll(`'${from}'`, `'${to}'`);
	jsContents = jsContents.replaceAll(`"${from}"`, `"${to}"`);
}
await Bun.write(js, jsContents);
