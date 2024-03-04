export async function asyncTimeout(timeout: number): Promise<void> {
	return new Promise((resolve) => {
		setTimeout(resolve, timeout);
	});
}

export function unreachable(value: never): never {
	throw new Error(`Unreachable: ${value}`);
}
