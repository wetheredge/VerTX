_default:
    @just --list --unsorted

clean:
    rm -r dist

fmt:
    pnpm biome format --write .

check:
    pnpm biome ci .
    pnpm tsc

# Run a local dev server. Needs the vhs-server dev server running
dev:
    pnpm vite

build: check
    pnpm vite build

# Run a local server from the latest build artifacts. Needs the vhs-server dev server running
preview:
    pnpm vite preview
