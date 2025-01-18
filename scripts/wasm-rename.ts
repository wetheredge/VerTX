#!/usr/bin/env bun

const baseDir = process.argv[2];
if (baseDir == null || baseDir === '') {
	throw new Error('must be called with base directory');
}

const stdin = await Bun.stdin.text();
const renames = new Map();
for (const line of stdin.split('\n')) {
	const [from, to] = line.split(' => ');
	if (to && from !== to) {
		renames.set(from, to);
	}
}

const base = `${baseDir}/vertx`;

const wasmDts = Bun.file(`${base}_bg.wasm.d.ts`);
let wasmDtsContents = await wasmDts.text();
for (const [from, to] of renames) {
	wasmDtsContents = wasmDtsContents.replaceAll(from, to);
}
await Bun.write(wasmDts, wasmDtsContents);

const dts = Bun.file(`${base}.d.ts`);
let dtsContents = await dts.text();
for (const [from, to] of renames) {
	dtsContents = dtsContents.replaceAll(`readonly ${from}`, `readonly ${to}`);
	dtsContents = dtsContents.replaceAll(`'${from}'`, `'${to}'`);
	dtsContents = dtsContents.replaceAll(`"${from}"`, `"${to}"`);
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
